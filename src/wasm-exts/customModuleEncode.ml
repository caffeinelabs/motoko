(*
This module originated as a copy of interpreter/binary/encode.ml in the
reference implementation.
With adjustments from memory64.

The changes are:
 * Support for writing out a source map for the Code parts
 * Support for additional custom sections
 * Manual selective support for bulk-memory operations `memory_copy` and `memory_fill` (WebAssembly/spec@7fa2f20).
 * Support for passive data segments (incl. `MemoryInit`).
 * Support for table index in `call_indirect` (reference-types proposal).

The code is otherwise as untouched as possible, so that we can relatively
easily apply diffs from the original code (possibly manually).
 *)

(* Note [funneling DIEs through Wasm.Ast]
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
The DWARF debugging information entry (DIE) is a simple data carrier meant
to be transmitted in a sequential fashion. Here, DIEs are attached to
specially crafted instructions (Meta) in the instruction stream
that is derived from the Wasm.Ast. Since these instructions are inserted artificially
and are not intended for execution, they will not be emitted as instructions, but
aggregated, correlated and finally output into DWARF sections of the binary.
DIEs are defined in Dwarf5.Meta and can be recognised via the `is_dwarf_like` predicate.
When extracted from the instruction stream using the predicate, we can check whether
they are a tag (pre-filled) with attributes/subtags or free-standing attributes that will
end up in the last tag. Similarly later tags nest into open tags. The larger-scale hierarchical
structure is finally restored when all instructions are emitted. The mechanism is described in
the blog post http://eli.thegreenplace.net/2011/09/29/an-interesting-tree-serialization-algorithm-from-dwarf

 *)


(* Note [bubbling up types in the tag hierarchy]
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
Certain DIEs (precisely `DW_TAG_*`s) can be referenced from attributes
by means of position in the `.debug_info` section. E.g. types are referenced
from a variety of DIEs. But since we generate DIEs for types on the fly,
they end up at the same hierarchy level as the referencing DIE. Such references
are allocated serially by extending a mapping to promises. The promise gets
fulfilled when the prerequisite DIE is externalised into the section.
To have every referencable tag a fulfilled section position, on the tag closing
trigger we move every referencable DIE out of it and into the parent, effectively
bubbling all up to toplevel. Then, immediately before externalising the DIE tree,
we perform a stable sort by serial number, with non-referencable DIEs trailing.

 *)

(* Note [placeholder promises for typedefs]
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
When forming the DIE for a Motoko type synonym (`type List = ...`)
we need to do something special. Since such typedefs are cycle-breakers
in the type system, we will need to adopt the same property in the
`.debug_info` section too. So, we'll output `DW_TAG_typedef` before
even knowing which type it refers to. Instead we use a DW_FORM_ref4
for its `DW_AT_type` attribute, which is backpatchable. The value of this
attribute is an integer, pointing to a fulfilled promise created when the
DIE was formed. It got fulfilled when the typedef's type became known,
another DIE, formed shortly after the typedef's. Resolving this fulfilled
promise in turn gives us the index (actual type ref) of an unfulfilled
promise (the forward reference). This forward reference will be fulfilled
to be a byte offset in the section as soon as the corresponding DIE is emitted.

We keep a function that performs the patching of the section (before it is
written to disk) by overwriting the preliminary bytes in `DW_TAG_typedef`'s
`DW_AT_type` with the now fulfilled offset obtained from the forward reference.

 *)

module Promise = Lib.Promise

open Dwarf5.Meta

open CustomModule

(* VLQ Base64 encoder for source maps.
   Derived from https://github.com/flowtype/ocaml-vlq (MIT license).
   Copyright (c) 2018-present, Facebook, Inc.
   Only the encode path is included; decode (which depends on the
   removed-in-OCaml-5 Stream module) is not needed by moc. *)
module Vlq_base64 : sig
  val encode : Buffer.t -> int -> unit
end = struct
  let shift = 5
  let vlq_base = 1 lsl shift
  let vlq_base_mask = vlq_base - 1
  let vlq_continuation_bit = vlq_base
  let base64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  let char_of_digit d = base64.[d]

  let rec encode_vlq buf vlq =
    let digit = vlq land vlq_base_mask in
    let vlq = vlq lsr shift in
    if vlq = 0 then Buffer.add_char buf (char_of_digit digit)
    else begin
      Buffer.add_char buf (char_of_digit (digit lor vlq_continuation_bit));
      encode_vlq buf vlq
    end

  let encode buf value =
    let vlq = if value < 0 then ((-value) lsl 1) + 1 else value lsl 1 in
    encode_vlq buf vlq
end

(* Version *)

let version = 1l


(* Errors *)

module Code = Wasm.Error.Make ()

(* Encoding stream *)

type stream =
{
  buf : Buffer.t;
  patches : (int * char) list ref
}

let stream () = {buf = Buffer.create 8192; patches = ref []}
let pos s = Buffer.length s.buf
let put s b = Buffer.add_char s.buf b
let put_string s bs = Buffer.add_string s.buf bs
let patch s pos b = s.patches := (pos, b) :: !(s.patches)

let to_string s =
  let bs = Buffer.to_bytes s.buf in
  List.iter (fun (pos, b) -> Bytes.set bs pos b) !(s.patches);
  Bytes.to_string bs

module References = Map.Make (struct type t = int let compare = compare end)

let dw_references = ref References.empty
let num_dw_references = ref 1 (* 0 would mean: "this tag doesn't fulfill a reference" *)
let promise_reference_slot p =
  let have = !num_dw_references in
  dw_references := References.add have p !dw_references;
  num_dw_references := 1 + have;
  have
let allocate_reference_slot () =
  promise_reference_slot (Promise.make ())

(* Encoding *)

let encode (em : extended_module) =
  let s = stream () in

  (* source map *)
  let map = Buffer.create 0 in
  let sources = ref [] in
  let sourcesContent = ref [] in
  let segs = ref 0 in
  let prev_if = ref 0 in
  let prev_ol = ref 0 in
  let prev_oc = ref 0 in
  let prev_il = ref 0 in
  let prev_ic = ref 0 in

  (* modify reference *)
  let modif r f = r := f !r in

  let rec add_source filename = function (* FIXME: use add_string *)
    | [] ->
      sources := !sources @ [ filename ];
      let source_code =
        try
          if filename = "prelude" then Prelude.prelude else
          if filename = "prim" then Prelude.prim_module  ~timers:!Mo_config.Flags.global_timer else
          (*
          Here we opportunistically see if we can find the source file
          mentioned in the source location, and if we can, include its source
          verbatim in the source map
          *)
          let ic = Stdlib.open_in filename in
          let n = in_channel_length ic in
          let s = Bytes.create n in
          really_input ic s 0 n;
          close_in ic;
          Bytes.to_string s
        with _ -> "" in
      sourcesContent := !sourcesContent @ [ source_code ];
      0
    | h :: t -> if filename = h then 0 else 1 + add_source filename t
  in

  let add_string gen strings str = (* FIXME: perform suffix compression? *)
    let strs = !strings in
    match List.assoc_opt str strs with
    | Some v -> v
    | _ ->
      let v = gen strs in
      let strs' = (str, v) :: strs in strings := strs'; v
  in

  let dwarf_strings = ref [] in

  let add_dwarf_string =
    add_string (function | [] -> 0 | (h, p) :: _ -> String.length h + 1 + p) dwarf_strings in

  let module Instrs = Set.Make (struct type t = int * Source.pos let compare = compare end) in
  let statement_positions = ref Instrs.empty in

  let module DW_Sequence = Set.Make (struct type t = int * Instrs.t * int let compare = compare end) in
  let sequence_bounds = ref DW_Sequence.empty in

  let dwarf_tags = ref [Tag (None, 0, [])] in

  let is_closed tag =
    let tag_of (t', _, _) = tag = t' in
    let (_, has_children, _) = List.find tag_of Abbreviation.abbreviations in
    not (has_children <> 0)
  in

  let code_section_start = ref 0 in

  let add_dwarf_tag refi tag =
    let position_attr =
      if tag = Dwarf5.dw_TAG_lexical_block then
        [IntAttribute (Dwarf5.dw_AT_low_pc, pos s - !code_section_start)]
      else [] in

    (* invariant: at most one closed tag waiting for attributes *)

    dwarf_tags :=
      match !dwarf_tags with
      | [Tag (refi, t, viscera)] when tag = Dwarf5.dw_TAG_compile_unit && t = 0 ->
        [Tag (refi, tag, viscera)]
      | Tag (_, t', _) as closed :: Tag (r, t, arts) :: tail when is_closed t' ->
        Tag (refi, tag, position_attr) :: Tag (r, t, closed :: arts) :: tail
      | tail ->
        Tag (refi, tag, position_attr) :: tail in

  let no_tags = List.for_all (function Tag _ -> false | _ -> true) in
  let hoistables = List.partition (function Tag (Some r, _, _) -> true | _ -> false) in

  let rec close_dwarf genuine =
    (* hoist out referencable tags,
       see Note [bubbling up types in the tag hierarchy] *)
    begin match !dwarf_tags with
    | Tag (refi, t, viscera) :: Tag (refi', t', viscera') :: tail
        when genuine && Dwarf5.(dw_TAG_subprogram = t land 0xFF || dw_TAG_lexical_block = t land 0xFF) ->
      let hoist, stay = hoistables viscera in
      dwarf_tags := Tag (refi, t, stay) :: Tag (refi', t', hoist @ viscera') :: tail
    | _ -> ()
    end;

    match !dwarf_tags with
    | Tag (_, t, viscera) :: tail when Dwarf5.dw_TAG_lexical_block = t && no_tags viscera ->
      dwarf_tags := tail
    | Tag (refi, t, viscera) :: tail when genuine && Dwarf5.dw_TAG_lexical_block = t ->
      dwarf_tags := Tag (refi, t, IntAttribute (Dwarf5.dw_AT_high_pc, pos s - !code_section_start) :: viscera) :: tail;
      close_dwarf false
    | [] -> failwith "no open DW_TAG"
    | [Tag (None, t, viscera)] when Dwarf5.dw_TAG_compile_unit = t ->
      dwarf_tags := Tag (None, t, viscera) :: []
    | [Tag _] -> failwith "TOPLEVEL: NOT NESTING"
    | Tag (_, t', _) as closed :: Tag (r, t, arts) :: tail when is_closed t' ->
      dwarf_tags := Tag (r, t, closed :: arts) :: tail; close_dwarf genuine
    | Tag (_, t', _) as nested :: Tag (r, tag, arts) :: t ->
      dwarf_tags := Tag (r, tag, nested :: arts) :: t
    | _ -> failwith "cannot close DW_AT" in
  let add_dwarf_attribute attr =
    dwarf_tags := match !dwarf_tags with
                  | Tag (r, tag, arts) :: t -> Tag (r, tag, attr :: arts) :: t
                  | _ -> assert false
  in

  (* keeping count of the DWARF code sequence we are in *)
  let sequence_number = ref 0 in
  (* array of the sizes of the emitted subprograms *)
  let subprogram_sizes = Promise.make () in
  (* offset of the range lists section *)
  let rangelists = Promise.make () in

  let extract_dwarf refi tag =
    let open Dwarf5 in
    let extract = function
      | OffsetAttribute at when at = dw_AT_low_pc && tag = dw_TAG_compile_unit ->
        add_dwarf_attribute (IntAttribute (at, 0))
      | OffsetAttribute at when at = dw_AT_low_pc && tag land 0xFF = dw_TAG_subprogram ->
        add_dwarf_attribute (IntAttribute (at, !sequence_number))
      | OffsetAttribute at when at = dw_AT_high_pc && tag land 0xFF = dw_TAG_subprogram ->
        let s = !sequence_number in
        let resolve () = IntAttribute (at, Array.get (Promise.value subprogram_sizes) s) in
        add_dwarf_attribute (FutureAttribute resolve)
      | OffsetAttribute at when at = dw_AT_ranges ->
        let resolve () = IntAttribute (at, Promise.value rangelists) in
        add_dwarf_attribute (FutureAttribute resolve) (* see Note [Low_pc, High_pc, Ranges are special] *)
      | (IntAttribute _ | StringAttribute _) as attr ->
        add_dwarf_attribute attr
      | _ -> assert false
    in
    add_dwarf_tag refi tag;
    List.iter extract
  in

  let add_to_map file il ic ol oc =
    let il = il - 1 in
    let if_ = add_source file !sources in
    if ol <> !prev_ol then Buffer.add_char map ';';
    Vlq_base64.encode map (oc - !prev_oc);             (* output column *)
    Vlq_base64.encode map (if_ - !prev_if);            (* sources index *)
    Vlq_base64.encode map (il - !prev_il);             (* input row *)
    Vlq_base64.encode map (ic - !prev_ic);             (* input column *)
    Buffer.add_char map ',';

    prev_if := if_; prev_ol := ol; prev_oc := oc; prev_il := il; prev_ic := ic; incr segs
  in

  let module E = struct
    (* Generic values *)

    let u8 i = put s (Char.chr (i land 0xff))
    let u16 i = u8 (i land 0xff); u8 (i lsr 8)
    let u32 i =
      Int32.(u16 (to_int (logand i 0xffffl));
             u16 (to_int (shift_right i 16)))
    let u64 i =
      Int64.(u32 (to_int32 (logand i 0xffffffffL));
             u32 (to_int32 (shift_right i 32)))

    let rec vu64 i =
      let b = Int64.(to_int (logand i 0x7fL)) in
      if 0L <= i && i < 128L then u8 b
      else (u8 (b lor 0x80); vu64 (Int64.shift_right_logical i 7))

    let rec vs64 i =
      let b = Int64.(to_int (logand i 0x7fL)) in
      if -64L <= i && i < 64L then u8 b
      else (u8 (b lor 0x80); vs64 (Int64.shift_right i 7))

    let vu32 i = vu64 Int64.(logand (of_int32 i) 0xffffffffL)
    let vs7 i = vs64 (Int64.of_int i)
    let vs32 i = vs64 (Int64.of_int32 i)
    let vs33 i = vs64 (Wasm.I64_convert.extend_i32_s i)
    let f32 x = u32 (Wasm.F32.to_bits x)
    let f64 x = u64 (Wasm.F64.to_bits x)

    let flag b i = if b then 1 lsl i else 0

    let len i =
      if Int32.to_int (Int32.of_int i) <> i then
        Code.error Source.no_region
          "cannot encode length with more than 32 bit";
      vu32 (Int32.of_int i)

    let string bs = len (String.length bs); put_string s bs
    let name n = string (Lib.Utf8.encode n)
    let list f xs = List.iter f xs
    let opt f xo = Option.iter f xo
    let vec_by l f xs = l (List.length xs); list f xs
    let vec f xs = vec_by len f xs

    let gap32 () = let p = pos s in u32 0l; u8 0; p
    let patch_gap32 p n =
      assert (n <= 0x0fff_ffff); (* Strings cannot excess 2G anyway *)
      let lsb i = Char.chr (i land 0xff) in
      patch s p (lsb (n lor 0x80));
      patch s (p + 1) (lsb ((n lsr 7) lor 0x80));
      patch s (p + 2) (lsb ((n lsr 14) lor 0x80));
      patch s (p + 3) (lsb ((n lsr 21) lor 0x80));
      patch s (p + 4) (lsb (n lsr 28))

    (* Types *)

    open Types

    let value_type = function
      | I32Type -> vs7 (-0x01)
      | I64Type -> vs7 (-0x02)
      | F32Type -> vs7 (-0x03)
      | F64Type -> vs7 (-0x04)

    let elem_type = function
      | FuncRefType -> vs7 (-0x10)

    let stack_type = vec value_type
    let func_type = function
      | FuncType (ins, out) -> vs7 (-0x20); stack_type ins; stack_type out

    let limits vu {min; max} it =
      let flags = flag (max <> None) 0 + flag (it = I64IndexType) 2 in
      u8 flags; vu min; opt vu max

    let table_type = function
      | TableType (lim, t) -> elem_type t; limits vu32 lim I32IndexType
      | HugeTableType (lim, t) -> elem_type t; limits vu64 lim I64IndexType

    let memory_type = function
      | MemoryType (lim, it) -> limits vu64 lim it

    let mutability = function
      | Immutable -> u8 0
      | Mutable -> u8 1

    let global_type = function
      | GlobalType (t, mut) -> value_type t; mutability mut

    (* Expressions *)

    open Ast
    open Values
    open Wasm.Source

    let op n = u8 n
    let end_ () = op 0x0b

    let memop {align; offset; _} = vu32 (Int32.of_int align); vu64 offset

    let var x = vu32 x.it

    let block_type = function
      | VarBlockType x -> vs33 x.it
      | ValBlockType None -> vs7 (-0x40)
      | ValBlockType (Some t) -> value_type t

    let rec instr noting e =
      if e.at <> no_region then add_to_map e.at.left.file e.at.left.line e.at.left.column 0 (pos s);
      noting e;
      let instr = instr noting in

      match e.it with
      | Meta TagClose -> close_dwarf true
      | Meta (StatementDelimiter left) ->
        modif statement_positions (Instrs.add (pos s, left))
      | Meta (Tag (r, t, attrs_tags)) ->
        let tags, attrs = List.partition (function Tag _ | Grouped _ -> true | _ -> false) attrs_tags in
        extract_dwarf r t attrs;
        List.iter (fun t -> instr { e with it = (Meta t) }) tags
      | Meta (Grouped []) -> ()
      | Meta (Grouped (late :: former)) ->
        instr { e with it = Meta (Grouped former) };
        instr { e with it = Meta late }
      | Meta _ -> assert false

      | Unreachable -> op 0x00
      | Nop -> op 0x01

      | Block (bt, es) -> op 0x02; block_type bt; list instr es; end_ ()
      | Loop (bt, es) -> op 0x03; block_type bt; list instr es; end_ ()
      | If (bt, es1, es2) ->
        op 0x04; block_type bt; list instr es1;
        if es2 <> [] then op 0x05;
        list instr es2; end_ ()

      | Br x -> op 0x0c; var x
      | BrIf x -> op 0x0d; var x
      | BrTable (xs, x) -> op 0x0e; vec var xs; var x
      | Return -> op 0x0f
      | Call x -> op 0x10; var x
      | CallIndirect (x, y) -> op 0x11; var y; var x

      | Drop -> op 0x1a
      | Select -> op 0x1b

      | LocalGet x -> op 0x20; var x
      | LocalSet x -> op 0x21; var x
      | LocalTee x -> op 0x22; var x
      | GlobalGet x -> op 0x23; var x
      | GlobalSet x -> op 0x24; var x

      | Load ({ty = I32Type; sz = None; _} as mo) -> op 0x28; memop mo
      | Load ({ty = I64Type; sz = None; _} as mo) -> op 0x29; memop mo
      | Load ({ty = F32Type; sz = None; _} as mo) -> op 0x2a; memop mo
      | Load ({ty = F64Type; sz = None; _} as mo) -> op 0x2b; memop mo
      | Load ({ty = I32Type; sz = Some (Pack8, SX); _} as mo) ->
        op 0x2c; memop mo
      | Load ({ty = I32Type; sz = Some (Pack8, ZX); _} as mo) ->
        op 0x2d; memop mo
      | Load ({ty = I32Type; sz = Some (Pack16, SX); _} as mo) ->
        op 0x2e; memop mo
      | Load ({ty = I32Type; sz = Some (Pack16, ZX); _} as mo) ->
        op 0x2f; memop mo
      | Load {ty = I32Type; sz = Some (Pack32, _); _} ->
        assert false
      | Load ({ty = I64Type; sz = Some (Pack8, SX); _} as mo) ->
        op 0x30; memop mo
      | Load ({ty = I64Type; sz = Some (Pack8, ZX); _} as mo) ->
        op 0x31; memop mo
      | Load ({ty = I64Type; sz = Some (Pack16, SX); _} as mo) ->
        op 0x32; memop mo
      | Load ({ty = I64Type; sz = Some (Pack16, ZX); _} as mo) ->
        op 0x33; memop mo
      | Load ({ty = I64Type; sz = Some (Pack32, SX); _} as mo) ->
        op 0x34; memop mo
      | Load ({ty = I64Type; sz = Some (Pack32, ZX); _} as mo) ->
        op 0x35; memop mo
      | Load {ty = F32Type | F64Type; sz = Some _; _} ->
        assert false

      | Store ({ty = I32Type; sz = None; _} as mo) -> op 0x36; memop mo
      | Store ({ty = I64Type; sz = None; _} as mo) -> op 0x37; memop mo
      | Store ({ty = F32Type; sz = None; _} as mo) -> op 0x38; memop mo
      | Store ({ty = F64Type; sz = None; _} as mo) -> op 0x39; memop mo
      | Store ({ty = I32Type; sz = Some Pack8; _} as mo) -> op 0x3a; memop mo
      | Store ({ty = I32Type; sz = Some Pack16; _} as mo) -> op 0x3b; memop mo
      | Store {ty = I32Type; sz = Some Pack32; _} -> assert false
      | Store ({ty = I64Type; sz = Some Pack8; _} as mo) -> op 0x3c; memop mo
      | Store ({ty = I64Type; sz = Some Pack16; _} as mo) -> op 0x3d; memop mo
      | Store ({ty = I64Type; sz = Some Pack32; _} as mo) -> op 0x3e; memop mo
      | Store {ty = F32Type | F64Type; sz = Some _; _} -> assert false

      | MemorySize -> op 0x3f; u8 0x00
      | MemoryGrow -> op 0x40; u8 0x00

      (* Manual extension for bulk-memory operations *)
      | MemoryFill -> op 0xfc; vu32 0x0bl; u8 0x00
      | MemoryCopy -> op 0xfc; vu32 0x0al; u8 0x00; u8 0x00
      (* End of manual extension *)
      (* Manual extension for passive data segments *)
      | MemoryInit x -> op 0xfc; vu32 0x08l; var x; u8 0x00
      (* End of manual extension *)

      | Const {it = I32 c; _} -> op 0x41; vs32 c
      | Const {it = I64 c; _} -> op 0x42; vs64 c
      | Const {it = F32 c; _} -> op 0x43; f32 c
      | Const {it = F64 c; _} -> op 0x44; f64 c

      | Test (I32 I32Op.Eqz) -> op 0x45
      | Test (I64 I64Op.Eqz) -> op 0x50
      | Test (F32 _) -> assert false
      | Test (F64 _) -> assert false

      | Compare (I32 I32Op.Eq) -> op 0x46
      | Compare (I32 I32Op.Ne) -> op 0x47
      | Compare (I32 I32Op.LtS) -> op 0x48
      | Compare (I32 I32Op.LtU) -> op 0x49
      | Compare (I32 I32Op.GtS) -> op 0x4a
      | Compare (I32 I32Op.GtU) -> op 0x4b
      | Compare (I32 I32Op.LeS) -> op 0x4c
      | Compare (I32 I32Op.LeU) -> op 0x4d
      | Compare (I32 I32Op.GeS) -> op 0x4e
      | Compare (I32 I32Op.GeU) -> op 0x4f

      | Compare (I64 I64Op.Eq) -> op 0x51
      | Compare (I64 I64Op.Ne) -> op 0x52
      | Compare (I64 I64Op.LtS) -> op 0x53
      | Compare (I64 I64Op.LtU) -> op 0x54
      | Compare (I64 I64Op.GtS) -> op 0x55
      | Compare (I64 I64Op.GtU) -> op 0x56
      | Compare (I64 I64Op.LeS) -> op 0x57
      | Compare (I64 I64Op.LeU) -> op 0x58
      | Compare (I64 I64Op.GeS) -> op 0x59
      | Compare (I64 I64Op.GeU) -> op 0x5a

      | Compare (F32 F32Op.Eq) -> op 0x5b
      | Compare (F32 F32Op.Ne) -> op 0x5c
      | Compare (F32 F32Op.Lt) -> op 0x5d
      | Compare (F32 F32Op.Gt) -> op 0x5e
      | Compare (F32 F32Op.Le) -> op 0x5f
      | Compare (F32 F32Op.Ge) -> op 0x60

      | Compare (F64 F64Op.Eq) -> op 0x61
      | Compare (F64 F64Op.Ne) -> op 0x62
      | Compare (F64 F64Op.Lt) -> op 0x63
      | Compare (F64 F64Op.Gt) -> op 0x64
      | Compare (F64 F64Op.Le) -> op 0x65
      | Compare (F64 F64Op.Ge) -> op 0x66

      | Unary (I32 I32Op.Clz) -> op 0x67
      | Unary (I32 I32Op.Ctz) -> op 0x68
      | Unary (I32 I32Op.Popcnt) -> op 0x69
      | Unary (I32 (I32Op.ExtendS Pack8)) -> op 0xc0
      | Unary (I32 (I32Op.ExtendS Pack16)) -> op 0xc1
      | Unary (I32 (I32Op.ExtendS Pack32)) -> assert false

      | Unary (I64 I64Op.Clz) -> op 0x79
      | Unary (I64 I64Op.Ctz) -> op 0x7a
      | Unary (I64 I64Op.Popcnt) -> op 0x7b
      | Unary (I64 (I64Op.ExtendS Pack8)) -> op 0xc2
      | Unary (I64 (I64Op.ExtendS Pack16)) -> op 0xc3
      | Unary (I64 (I64Op.ExtendS Pack32)) -> op 0xc4

      | Unary (F32 F32Op.Abs) -> op 0x8b
      | Unary (F32 F32Op.Neg) -> op 0x8c
      | Unary (F32 F32Op.Ceil) -> op 0x8d
      | Unary (F32 F32Op.Floor) -> op 0x8e
      | Unary (F32 F32Op.Trunc) -> op 0x8f
      | Unary (F32 F32Op.Nearest) -> op 0x90
      | Unary (F32 F32Op.Sqrt) -> op 0x91

      | Unary (F64 F64Op.Abs) -> op 0x99
      | Unary (F64 F64Op.Neg) -> op 0x9a
      | Unary (F64 F64Op.Ceil) -> op 0x9b
      | Unary (F64 F64Op.Floor) -> op 0x9c
      | Unary (F64 F64Op.Trunc) -> op 0x9d
      | Unary (F64 F64Op.Nearest) -> op 0x9e
      | Unary (F64 F64Op.Sqrt) -> op 0x9f

      | Binary (I32 I32Op.Add) -> op 0x6a
      | Binary (I32 I32Op.Sub) -> op 0x6b
      | Binary (I32 I32Op.Mul) -> op 0x6c
      | Binary (I32 I32Op.DivS) -> op 0x6d
      | Binary (I32 I32Op.DivU) -> op 0x6e
      | Binary (I32 I32Op.RemS) -> op 0x6f
      | Binary (I32 I32Op.RemU) -> op 0x70
      | Binary (I32 I32Op.And) -> op 0x71
      | Binary (I32 I32Op.Or) -> op 0x72
      | Binary (I32 I32Op.Xor) -> op 0x73
      | Binary (I32 I32Op.Shl) -> op 0x74
      | Binary (I32 I32Op.ShrS) -> op 0x75
      | Binary (I32 I32Op.ShrU) -> op 0x76
      | Binary (I32 I32Op.Rotl) -> op 0x77
      | Binary (I32 I32Op.Rotr) -> op 0x78

      | Binary (I64 I64Op.Add) -> op 0x7c
      | Binary (I64 I64Op.Sub) -> op 0x7d
      | Binary (I64 I64Op.Mul) -> op 0x7e
      | Binary (I64 I64Op.DivS) -> op 0x7f
      | Binary (I64 I64Op.DivU) -> op 0x80
      | Binary (I64 I64Op.RemS) -> op 0x81
      | Binary (I64 I64Op.RemU) -> op 0x82
      | Binary (I64 I64Op.And) -> op 0x83
      | Binary (I64 I64Op.Or) -> op 0x84
      | Binary (I64 I64Op.Xor) -> op 0x85
      | Binary (I64 I64Op.Shl) -> op 0x86
      | Binary (I64 I64Op.ShrS) -> op 0x87
      | Binary (I64 I64Op.ShrU) -> op 0x88
      | Binary (I64 I64Op.Rotl) -> op 0x89
      | Binary (I64 I64Op.Rotr) -> op 0x8a

      | Binary (F32 F32Op.Add) -> op 0x92
      | Binary (F32 F32Op.Sub) -> op 0x93
      | Binary (F32 F32Op.Mul) -> op 0x94
      | Binary (F32 F32Op.Div) -> op 0x95
      | Binary (F32 F32Op.Min) -> op 0x96
      | Binary (F32 F32Op.Max) -> op 0x97
      | Binary (F32 F32Op.CopySign) -> op 0x98

      | Binary (F64 F64Op.Add) -> op 0xa0
      | Binary (F64 F64Op.Sub) -> op 0xa1
      | Binary (F64 F64Op.Mul) -> op 0xa2
      | Binary (F64 F64Op.Div) -> op 0xa3
      | Binary (F64 F64Op.Min) -> op 0xa4
      | Binary (F64 F64Op.Max) -> op 0xa5
      | Binary (F64 F64Op.CopySign) -> op 0xa6

      | Convert (I32 I32Op.ExtendSI32) -> assert false
      | Convert (I32 I32Op.ExtendUI32) -> assert false
      | Convert (I32 I32Op.WrapI64) -> op 0xa7
      | Convert (I32 I32Op.TruncSF32) -> op 0xa8
      | Convert (I32 I32Op.TruncUF32) -> op 0xa9
      | Convert (I32 I32Op.TruncSF64) -> op 0xaa
      | Convert (I32 I32Op.TruncUF64) -> op 0xab
      | Convert (I32 I32Op.TruncSatSF32) -> op 0xfc; op 0x00
      | Convert (I32 I32Op.TruncSatUF32) -> op 0xfc; op 0x01
      | Convert (I32 I32Op.TruncSatSF64) -> op 0xfc; op 0x02
      | Convert (I32 I32Op.TruncSatUF64) -> op 0xfc; op 0x03
      | Convert (I32 I32Op.ReinterpretFloat) -> op 0xbc

      | Convert (I64 I64Op.ExtendSI32) -> op 0xac
      | Convert (I64 I64Op.ExtendUI32) -> op 0xad
      | Convert (I64 I64Op.WrapI64) -> assert false
      | Convert (I64 I64Op.TruncSF32) -> op 0xae
      | Convert (I64 I64Op.TruncUF32) -> op 0xaf
      | Convert (I64 I64Op.TruncSF64) -> op 0xb0
      | Convert (I64 I64Op.TruncUF64) -> op 0xb1
      | Convert (I64 I64Op.TruncSatSF32) -> op 0xfc; op 0x04
      | Convert (I64 I64Op.TruncSatUF32) -> op 0xfc; op 0x05
      | Convert (I64 I64Op.TruncSatSF64) -> op 0xfc; op 0x06
      | Convert (I64 I64Op.TruncSatUF64) -> op 0xfc; op 0x07
      | Convert (I64 I64Op.ReinterpretFloat) -> op 0xbd

      | Convert (F32 F32Op.ConvertSI32) -> op 0xb2
      | Convert (F32 F32Op.ConvertUI32) -> op 0xb3
      | Convert (F32 F32Op.ConvertSI64) -> op 0xb4
      | Convert (F32 F32Op.ConvertUI64) -> op 0xb5
      | Convert (F32 F32Op.PromoteF32) -> assert false
      | Convert (F32 F32Op.DemoteF64) -> op 0xb6
      | Convert (F32 F32Op.ReinterpretInt) -> op 0xbe

      | Convert (F64 F64Op.ConvertSI32) -> op 0xb7
      | Convert (F64 F64Op.ConvertUI32) -> op 0xb8
      | Convert (F64 F64Op.ConvertSI64) -> op 0xb9
      | Convert (F64 F64Op.ConvertUI64) -> op 0xba
      | Convert (F64 F64Op.PromoteF32) -> op 0xbb
      | Convert (F64 F64Op.DemoteF64) -> assert false
      | Convert (F64 F64Op.ReinterpretInt) -> op 0xbf

      (* Custom encodings for emulating stable-memory, special cases
         of MemorySize, MemoryGrow and MemoryCopy
         requiring wasm features bulk-memory and multi-memory
      *)
      | StableSize -> op 0x3f; u8 0x01
      | StableGrow -> op 0x40; u8 0x01
      | StableRead -> op 0xfc; vu32 0x0al; u8 0x00; u8 0x01
      | StableWrite -> op 0xfc; vu32 0x0al; u8 0x01; u8 0x00

    let const c =
      list (instr ignore) c.it; end_ ()

    (* Sections *)

    let section id f x needed =
      if needed then begin
        u8 id;
        let g = gap32 () in
        let p = pos s in
        f x;
        patch_gap32 g (pos s - p)
      end

    let custom_section name f x needed =
      section 0 (fun x ->
        string name;
        f x
      ) x needed

    (* Type section *)
    let type_ t = func_type t.it

    let type_section ts =
      section 1 (vec type_) ts (ts <> [])

    (* Import section *)
    let import_desc d =
      match d.it with
      | FuncImport x -> u8 0x00; var x
      | TableImport t -> u8 0x01; table_type t
      | MemoryImport t -> u8 0x02; memory_type t
      | GlobalImport t -> u8 0x03; global_type t

    let import im =
      let {module_name; item_name; idesc} = im.it in
      name module_name; name item_name; import_desc idesc

    let import_section ims =
      section 2 (vec import) ims (ims <> [])

    (* Function section *)
    let func f = var f.it.ftype

    let func_section fs =
      section 3 (vec func) fs (fs <> [])

    (* Table section *)
    let table tab =
      let {ttype} = tab.it in
      table_type ttype

    let table_section tabs =
      section 4 (vec table) tabs (tabs <> [])

    (* Memory section *)
    let memory mem =
      let {mtype} = mem.it in
      memory_type mtype

    let memory_section mems =
      section 5 (vec memory) mems (mems <> [])

    (* Global section *)
    let global g =
      let {gtype; value} = g.it in
      global_type gtype; const value

    let global_section gs =
      section 6 (vec global) gs (gs <> [])

    (* Export section *)
    let export_desc d =
      match d.it with
      | FuncExport x -> u8 0; var x
      | TableExport x -> u8 1; var x
      | MemoryExport x -> u8 2; var x
      | GlobalExport x -> u8 3; var x

    let export ex =
      let {name = n; edesc} = ex.it in
      name n; export_desc edesc

    let export_section exs =
      section 7 (vec export) exs (exs <> [])

    (* Start section *)
    let start_section xo =
      section 8 (opt var) xo (xo <> None)

    (* Code section *)
    let compress ts =
      let combine t = function
        | (t', n) :: ts when t = t' -> (t, n + 1) :: ts
        | ts -> (t, 1) :: ts
      in List.fold_right combine ts []

    let local (t, n) = len n; value_type t

    let (here_dir, asset_dir) = (0, 1) (* reversed indices in dir_names, below *)
    let source_names =
      [ "prelude", (Promise.make (), asset_dir)
      ; "prim", (Promise.make (), asset_dir)
      ; "internals", (Promise.make (), asset_dir)
      ; "timers-api",  (Promise.make (), asset_dir)
      ; "rts.wasm", (Promise.make (), asset_dir) ] (* make these appear last in .debug_line file_name_entries *)

    (* Note: we use it like a key-value store and since file names are not unique, we need to add the directory index to the key *)
    let source_names = ref (List.map (fun (n, ((_, i) as t)) -> (n, i), t) source_names)

    let dir_names = (* ditto, but reversed: 6.2.4.1 Standard Content Descriptions *)
      ref [ "<moc-asset>", (Promise.make (), asset_dir)
          ; Filename.dirname "", (Promise.make (), here_dir) ]

    (* Initialize indices for source_names *)
    let source_path_indices =
      let n = List.length !source_names in
      ref (List.mapi (fun i ((p, _), (_, _)) -> p, n - 1 - i) !source_names)
    let add_source_name =
      let source_adder dir_index _ = Promise.make (), dir_index in
      let add_source_path_index (_, _) = function
        | "" -> assert false
        | str ->
          ignore (add_string (function [] -> assert false | (_, i) :: _ -> i + 1) source_path_indices str) in
      function
      | "" -> ()
      | ("prelude" | "prim" | "rts.wasm") as asset ->
        add_source_path_index (add_string (source_adder asset_dir) source_names (asset, asset_dir)) asset
      | path ->
        let dir, basename = Filename.(dirname path, basename path) in
        let _, dir_index = add_string (function [] -> assert false | (_, (_, i)) :: _ -> Promise.make (), i + 1) dir_names dir in
        let promise = add_string (source_adder dir_index) source_names (basename, dir_index) in
        add_source_path_index promise path

    let code f =
      let {locals; body; _} = f.it in
      let g = gap32 () in
      let p = pos s in
      vec local (compress locals);
      let instr_notes = ref Instrs.empty in
      let note i =
        if not (is_dwarf_like i.it) then
          (modif instr_notes (Instrs.add (pos s, i.at.left));
           ignore (add_source_name i.at.left.file)
          ) in
      list (instr note) body;
      modif instr_notes (Instrs.add (pos s, f.at.right));
      ignore (add_source_name f.at.right.file);
      end_ ();
      incr sequence_number;
      let sequence_end = pos s in
      patch_gap32 g (sequence_end - p);
      modif sequence_bounds (DW_Sequence.add (p, !instr_notes, sequence_end))

    let code_section fs =
      section 10 (fun fs -> code_section_start := pos s; vec code fs) fs (fs <> [])

    (* Element section *)
    let segment dat seg =
      let {index; offset; init} = seg.it in
      var index; const offset; dat init

    let table_segment seg =
      segment (vec var) seg

    let elem_section elems =
      section 9 (vec table_segment) elems (elems <> [])

    (* Manual extension for passive data segments *)
    (* Data section *)

    let data seg =
      let {dinit; dmode} = seg.it in
      match dmode.it with
      | Passive ->
        vu32 0x01l; string dinit
      | Active {index; offset} when index.it = 0l ->
        vu32 0x00l; const offset; string dinit
      | Active {index; offset} ->
        vu32 0x02l; var index; const offset; string dinit
      | Declarative ->
        failwith "illegal declarative data segment"

    let data_section datas =
      section 11 (vec data) datas (datas <> [])

    (* Data count section *)

    let data_count_section datas m =
      section 12 len (List.length datas) (datas <> [])
    (* End of manual extension *)

    (* sourceMappingURL section *)

    let source_mapping_url_section smu =
      match smu with
      | Some smu -> custom_section "sourceMappingURL" string smu true
      | None -> ()

    (* Name section *)

    let assoc_list : 'a. ('a -> unit) -> (int32 * 'a) list -> unit = fun f xs ->
      vec (fun (li, x) -> vu32 li; f x)
          (List.sort (fun (i1,_) (i2,_) -> compare i1 i2) xs)

    let name_section ns =
      let name_section_body (ns : name_section) =
        (* module name section *)
        section 0 (opt string) ns.module_ (ns.module_ <> None);
        (* function names section *)
        section 1 (assoc_list string) ns.function_names  (ns.function_names <> []);
        (* locals names section *)
        section 2 (assoc_list (assoc_list string)) ns.locals_names  (ns.locals_names <> []) in

      custom_section "name" name_section_body ns
        (ns.module_ <> None || ns.function_names <> [] || ns.locals_names <> [])

    let icp_custom_section name f opt =
      match opt with
      | None -> ()
      | Some (is_public, x) ->
        section 0 (fun x ->
          string ("icp:"^ (if is_public then "public " else "private ") ^ name);
          f x
        ) x true

    (* Motoko custom section *)

    let motoko_section_body labels =
      section 0 (vec string) labels (labels <> [])

    let utf8 bs =
      ignore (Lib.Utf8.decode bs);  (* assert well-formedness *)
      put_string s bs

    let motoko_sections motoko =
      icp_custom_section "motoko:stable-types" utf8 motoko.stable_types;
      icp_custom_section "motoko:compiler" utf8 motoko.compiler;
      custom_section "motoko" motoko_section_body motoko.labels (motoko.labels <> []) (* TODO: make an icp_section *)

    let enhanced_orthogonal_persistence_section version =
      icp_custom_section "enhanced-orthogonal-persistence" utf8 version

    let candid_sections candid =
      icp_custom_section "candid:service" utf8 candid.service;
      icp_custom_section "candid:args" utf8 candid.args

    let wasm_features_section wasm_features =
      let text = String.concat "," wasm_features in
      custom_section "wasm_features" utf8 text (text <> "")

    let uleb128 n = vu64 (Int64.of_int n)
    let sleb128 n = vs64 (Int64.of_int n)
    let close_section () = u8 0x00
    let write16 = Buffer.add_int16_le s.buf
    let write32 i = Buffer.add_int32_le s.buf (Int32.of_int i)
    let zero_terminated str = put_string s str; u8 0
    let vec_uleb128 el = vec_by uleb128 el
    let writeBlock1 str = let len = String.length str in assert (len < 256); u8 len; put_string s str
    let writeBlockLEB str = uleb128 (String.length str); put_string s str
    let dw_gap32 () = let p = pos s in write32 0x0; p
    let dw_patch_gap32 p n =
      let lsb i = Char.chr (i land 0xff) in
      patch s p (lsb n);
      patch s (p + 1) (lsb (n lsr 8));
      patch s (p + 2) (lsb (n lsr 16));
      patch s (p + 3) (lsb (n lsr 24))
    let dw_patches = ref Fun.id

    let debug_abbrev_section () =
      let tag (t, ch, kvs) =
        uleb128 (t land 0xFFFF); u8 ch;
        assert (kvs <> []); (* these run risk of dead-code elimination *)
        List.iter (fun (k, v) -> uleb128 k; uleb128 v) kvs in
      let abbrev i abs = uleb128 (i + 1); tag abs; close_section (); close_section () in
      let section_body abs = List.iteri abbrev abs; close_section () in
      custom_section ".debug_abbrev" section_body Abbreviation.abbreviations true

    (* dw_FORM writers *)
    let writeForm : int -> die -> unit =
      let open Dwarf5 in
      function
      | f when dw_FORM_strp = f ->
        begin function
          | StringAttribute (attr, str) -> write32 (add_dwarf_string str)
          | _ -> failwith "dw_FORM_strp"
        end
      | f when dw_FORM_data1 = f ->
        begin function
          | IntAttribute (attr, i) -> u8 i
          | _ -> failwith "dw_FORM_data1"
        end
      | f when dw_FORM_data2 = f ->
        begin function
          | IntAttribute (attr, i) -> write16 i
          | _ -> failwith "dw_FORM_data2"
        end
      | f when dw_FORM_data4 = f ->
        begin function
          | IntAttribute (attr, i) -> write32 i
          | _ -> failwith "dw_FORM_data4"
        end
      | f when dw_FORM_addr = f ->
        begin function
          | IntAttribute (attr, i) -> write32 i
          | _ -> failwith "dw_FORM_addr"
        end
      | f when dw_FORM_addrx = f ->
        begin function
          | IntAttribute (attr, i) -> uleb128 i
          | _ -> failwith "dw_FORM_addrx"
        end
      | f when dw_FORM_ref4 = f ->
        begin function
          | IntAttribute (attr, i) ->
            (* See Note [placeholder promises for typedefs] *)
            let placeholder_promise = References.find i !dw_references in
            assert (Promise.is_fulfilled placeholder_promise);
            let forward_ref = Promise.value placeholder_promise in
            let offset_promise = References.find forward_ref !dw_references in
            if Promise.is_fulfilled offset_promise
            then write32 (Promise.value offset_promise)
            else dw_patches :=
                   (fun gap ps () ->
                     ps ();
                     dw_patch_gap32 gap (Promise.value offset_promise)
                   ) (dw_gap32 ()) !dw_patches
          | _ -> failwith "dw_FORM_ref_ref4"
        end
      | f when dw_FORM_ref_udata = f ->
        begin function
          | IntAttribute (attr, i) ->
            uleb128 (Promise.value (References.find i !dw_references))
          | _ -> failwith "dw_FORM_ref_udata"
        end
      | f when dw_FORM_sec_offset = f ->
        begin function
          | IntAttribute (attr, i) -> write32 i
          | _ -> failwith "dw_FORM_sec_offset"
        end
      | f when dw_FORM_block1 = f ->
        begin function
          | StringAttribute (attr, str) -> writeBlock1 str
          | _ -> failwith "dw_FORM_block1"
        end
      | f when dw_FORM_exprloc = f ->
        begin function
          | StringAttribute (attr, str) -> writeBlockLEB str
          | _ -> failwith "dw_FORM_exprloc"
        end
      | f when dw_FORM_flag = f ->
        begin function
          | IntAttribute (attr, b) -> u8 b
          | _ -> failwith "dw_FORM_flag"
        end
      | f when dw_FORM_flag_present = f ->
        begin function
          | IntAttribute (attr, 0) -> failwith "dw_FORM_flag_present with false"
          | IntAttribute (attr, _) -> ()
          | _ -> failwith "dw_FORM_flag_present"
        end
      | _ -> failwith "cannot write form"

    let info_section_start = ref 0

    let rec writeTag at_toplevel = function
      | Tag (r, t, contentsRevd) ->
        begin match r with
        | Some refi ->
          Promise.fulfill (References.find refi !dw_references) (pos s - !info_section_start)
        | None -> assert (t <> Dwarf5.dw_TAG_base_type)
        end;

        (* we have to be careful to only reference tags already written,
           so maintain creation order *)
        let ref_priority a b = match a, b with
          | Tag (Some m, _, _), Tag (Some n, _, _) -> compare n m
          | _, Tag (Some _, _, _) -> -1
          | Tag (Some _, _, _), _ -> 1
          | _ -> 0 in

        let contents = List.rev (if at_toplevel then List.stable_sort ref_priority contentsRevd else contentsRevd) in
        let wanted_tag (t', _, _) = t = t' in
        let (_, has_children, forms) = List.find wanted_tag Abbreviation.abbreviations in
        let rec pairing (attr, form) = function
          | Tag _ | TagClose | StatementDelimiter _ | Grouped _ -> failwith "Attribute expected"
          | FutureAttribute f ->
            pairing (attr, form) (f ())
          | StringAttribute (a, path0) when a = Dwarf5.dw_AT_decl_file ->
            let path = if path0 = "" then "prim" else path0 in
            if attr <> a then Printf.printf "DATA1 attr: 0x%x = a: 0x%x (in TAG 0x%x) PATH: %s  ULT: (%s, %d)\n" attr a t path    (fst (List.hd !source_path_indices)) (snd (List.hd !source_path_indices));
            assert (attr = a);
            writeForm form (IntAttribute (a, List.(snd (hd !source_path_indices) - assoc path !source_path_indices)))
          | IntAttribute (a, _) as art ->
            if attr <> a then Printf.printf "attr: 0x%x = a: 0x%x (in TAG 0x%x)\n" attr a t;
            assert (attr = a);
            writeForm form art
          | StringAttribute (a, _) as art -> assert (attr = a); writeForm form art
          | OffsetAttribute _ -> failwith "too late to resolve OffsetAttribute" in
        let rec indexOf cnt = function
          | h :: t when wanted_tag h -> cnt
          | _ :: t -> indexOf (cnt + 1) t
          | _ -> failwith "not encountered" in
        uleb128 (indexOf 1 Abbreviation.abbreviations);
        let nested_tags, attrs = List.partition (function Tag _ -> true | _ -> false) contents in

        List.iter2 pairing forms attrs;
        List.iter (writeTag false) nested_tags;
        if has_children <> 0 then close_section ()
      | _ -> failwith "Tag expected"

    let unit f =
      let g = dw_gap32 () in (* unit_length *)
      let p = pos s in
      f g; dw_patch_gap32 g (pos s - p)

    let debug_info_section () =
      let section_body abs =
        unit(fun info_start ->
            write16 0x0005; (* version *)
            u8 Dwarf5.dw_UT_compile; (* unit_type *)
            u8 4; (* address_size *)
            write32 0x0000; (* debug_abbrev_offset *)
            info_section_start := info_start;

            match !dwarf_tags with
            | [toplevel] -> writeTag true toplevel
            | _ -> failwith "expected one toplevel tag"
        );
        !dw_patches () in
      let relevant ts = ts <> [Tag (None, 0, [])] in
      custom_section ".debug_info" section_body dwarf_tags (relevant !dwarf_tags)

    let debug_strings_section dss =
      let rec debug_strings_section_body = function
        | [] -> ()
        | (h, _) :: t -> debug_strings_section_body t; zero_terminated h
      in
      custom_section ".debug_str" debug_strings_section_body dss (dss <> [])


    let debug_addr_section seqs =
      let debug_addr_section_body seqs =
        unit(fun start ->
            write16 0x0005; (* version *)
            u8 4; (* addr_size *)
            u8 0; (* segment_selector_size *)
            let write_addr (st, _, _) =
              let rel addr = addr - !code_section_start in
              write32 (rel st)
            in
            DW_Sequence.iter write_addr seqs;
        )
      in
      custom_section ".debug_addr" debug_addr_section_body seqs (not (DW_Sequence.is_empty seqs))


    (* 7.28 Range List Table *)
    let debug_rnglists_section sequence_bounds =
      let index = ref 0 in
      let debug_rnglists_section_body () =
        unit(fun start ->
            write16 0x0005; (* version *)
            u8 4; (* address_size *)
            u8 0; (* segment_selector_size *)
            write32 0; (* offset_entry_count *)

            Promise.fulfill rangelists (pos s - start);
            DW_Sequence.iter (fun (st, _, en) ->
                u8 Dwarf5.dw_RLE_startx_length;
                uleb128 !index;
                incr index;
                uleb128 (en - st))
              sequence_bounds;
            u8 Dwarf5.dw_RLE_end_of_list;

            (* extract the subprogram sizes to an array *)
            Promise.fulfill subprogram_sizes (Array.of_seq (Seq.map (fun (st, _, en) -> en - st) (DW_Sequence.to_seq sequence_bounds)))
        );

        in
      custom_section ".debug_rnglists" debug_rnglists_section_body () true

    (* Debug strings for line machine section, used by DWARF5: "6.2.4 The Line Number Program Header" *)

    let debug_line_str_section () =
      let debug_line_strings_section_body (dirs, sources) =
        let start = pos s in
        let rec strings = function
          | [] -> ()
          | (h, (p, _)) :: t ->
            Promise.fulfill p (pos s - start);
            zero_terminated h;
            strings t in
        strings dirs;
        strings sources in
      custom_section ".debug_line_str" debug_line_strings_section_body (!dir_names, List.map (fun ((n, _), p) -> n, p) !source_names) true

    (* Debug line machine section, see DWARF5: "6.2 Line Number Information" *)

    let debug_line_section fs =
      let debug_line_section_body () =

        unit(fun start ->
            (* see "6.2.4 The Line Number Program Header" *)
            write16 0x0005;
            u8 4;
            u8 0; (* segment_selector_size *)
            unit(fun _ ->
                u8 1; (* min_inst_length *)
                u8 1; (* max_ops_per_inst *)
                u8 (if Dwarf5.Machine.default_is_stmt then 1 else 0); (* default_is_stmt *)
                u8 0; (* line_base *)
                u8 12; (* line_range *)
                u8 13; (* opcode_base *)
                let open List in
                (* DW_LNS_copy .. DW_LNS_set_isa usage *)
                iter u8 [0; 1; 1; 1; 1; 0; 0; 0; 1; 0; 0; 1];

                let format (l, f) = uleb128 l; uleb128 f in
                let vec_format = vec_by u8 format in

                (* directory_entry_format_count, directory_entry_formats *)
                vec_format Dwarf5.[dw_LNCT_path, dw_FORM_line_strp];

                (* directories_count, directories *)
                vec_uleb128 write32 (rev_map (fun (_, (p, _)) -> Promise.value p) !dir_names);

                (* file_name_entry_format_count, file_name_entry_formats *)
                vec_format Dwarf5.[dw_LNCT_path, dw_FORM_line_strp; dw_LNCT_directory_index, dw_FORM_udata];

                (* The first entry in the sequence is the primary source file whose file name exactly
                   matches that given in the DW_AT_name attribute in the compilation unit debugging
                   information entry. This is ensured by the heuristics, that the last noted source file
                   will be placed at position 0 in the table *)
                vec_uleb128
                  (fun (pos, indx) -> write32 pos; uleb128 indx)
                  (map (fun (_, (p, dir_indx)) -> Promise.value p, dir_indx) !source_names);
            );

            (* build the statement loc -> addr map *)
            let statement_positions = !statement_positions in
            let module StmtsAt = Map.Make (struct type t = pos let compare = compare end) in
            let statements_at = StmtsAt.of_seq (Seq.map (fun (k, v) -> v, k) (Instrs.to_seq statement_positions)) in
            let is_statement_at (addr, loc) =
              match StmtsAt.find_opt loc statements_at with
              | Some addr' when addr = addr' -> true
              | _ -> false in

            (* generate the line section *)
            let code_start = !code_section_start in
            let rel addr = addr - code_start in
            let source_indices = !source_path_indices in

            let mapping epi (addr, {file; line; column} as loc) : Dwarf5.Machine.state =
              let file' = List.(snd (hd source_indices) - assoc (if file = "" then "prim" else file) source_indices) in
              let stmt = Instrs.mem loc statement_positions || is_statement_at loc (* FIXME TODO: why ||? *) in
              let addr' = rel addr in
              Dwarf5.Machine.{ ip = addr'; loc = { file = file'; line; col = column + 1 }; disc = 0; stmt; bb = false; mode = if addr' = epi then Epilogue else Regular }
            in

            let joining (prg, state) state' : int list list * Dwarf5.Machine.state =
              (* to avoid quadratic runtime, just collect (cons up) the partial lists here;
                 later we'll bring it in the right order and flatten *)
              Dwarf5.Machine.infer state state' :: prg, state'
            in

            let sequence (sta, notes, en) =
              let start, ending = rel sta, rel en in
              let notes_seq = Instrs.to_seq notes in
              let open Dwarf5.Machine in
              (* Decorate first instr, and prepend start address, non-statement (FIXME: clang says it *is* a statement) *)
              let seq_start_state = { start_state with ip = start; stmt = false } in
              let states_seq () =
                let open Seq in
                match map (mapping (ending - 1)) notes_seq () with
                | Nil -> failwith "there should be an 'end' instruction!"
                | Cons ({ip; _}, _) when ip = start -> failwith "at start already an instruction?"
                | Cons (state, _) as front ->
                  (* override default location from `start_state` *)
                  let start_state' = { seq_start_state with loc = state.loc } in
                  (* FIXME (4.11) use `cons` *)
                  Cons (start_state', fun () -> front)
              in

              let prg0, _ = Seq.fold_left joining ([], start_state) states_seq in
              let prg = List.fold_left (Fun.flip (@)) Dwarf5.[dw_LNS_advance_pc; 1; dw_LNE_end_sequence] prg0 in
              write_opcodes u8 uleb128 sleb128 write32 prg
            in
            DW_Sequence.iter sequence !sequence_bounds
        )
      in
      custom_section ".debug_line" debug_line_section_body () (fs <> [])


    (* Module *)

    let module_ (em : extended_module) =
      let m = em.module_ in

      u32 0x6d736100l;
      u32 version;
      (* no use-case for encoding dylink section yet, but here would be the place *)
      assert (em.dylink0 = []);
      type_section m.types;
      import_section m.imports;
      func_section m.funcs;
      table_section m.tables;
      memory_section m.memories;
      global_section m.globals;
      export_section m.exports;
      start_section m.start;
      elem_section m.elems;
      data_count_section m.datas m;
      code_section m.funcs;
      data_section m.datas;
      (* other optional sections *)
      name_section em.name;
      candid_sections em.candid;
      motoko_sections em.motoko;
      enhanced_orthogonal_persistence_section em.enhanced_orthogonal_persistence;
      wasm_features_section em.wasm_features;
      source_mapping_url_section em.source_mapping_url;
      if !Mo_config.Flags.debug_info then
        begin
          debug_abbrev_section ();
          debug_addr_section !sequence_bounds;
          debug_rnglists_section !sequence_bounds;
          debug_line_str_section ();
          debug_line_section m.funcs;
          debug_info_section ();
          debug_strings_section !dwarf_strings
        end
  end
  in E.module_ em;

  let mappings = Buffer.contents map in
  let n = max 0 ((String.length mappings) - 1) in
  let json : Yojson.Basic.t = `Assoc [
    ("version", `Int 3);
    ("sources", `List ( List.map (fun x -> `String x) !sources ) );
    ("sourcesContent", `List ( List.map (fun x -> if x = "" then `Null else `String x) !sourcesContent ) );
    ("mappings", `String (String.sub mappings 0 n) )
  ] in

  (Yojson.Basic.to_string json, to_string s)
