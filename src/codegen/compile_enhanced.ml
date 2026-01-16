(*
This module is the backend of the Motoko compiler. It takes a program in
the intermediate representation (ir.ml), and produces a WebAssembly module,
with Internet Computer extensions (customModule.ml). An important helper module is
instrList.ml, which provides a more convenient way of assembling WebAssembly
instruction lists, as it takes care of (1) source locations and (2) labels.

This file is split up in a number of modules, purely for namespacing and
grouping. Every module has a high-level prose comment explaining the concept;
this keeps documentation close to the code (a lesson learned from Simon PJ).
*)

open Ir_def
open Mo_values
open Mo_types
open Mo_config

open Wasm_exts.Ast
open Wasm_exts.Types
open Source

open Compile_common

(* WebAssembly pages are 64kb. *)
let page_size = Int64.of_int (64 * 1024)
let page_size_bits = 16

(* Scalar Tagging Scheme *)

(* Rationale:
   Scalar tags are variable length LSBs.
   A tag (in binary) is either
   * 10 for Int (leaving 30 bits payload)
   * 01(0+)0 for unsigned, e.g 0100 for Nat64
   * 11(0+)0 for signed,   e.g.1100 for Int64
   Types must be distinguished by tag.
   LSB must always be 0.
   Decoding the type of scalar is easy using `ctz` to count the trailing zeros, then
   switching on the MSB of the tag for sign (if any).
   We use the *longest* tag that accommodates the required payload bits, to allow room
   for any future tags that may require more payload bits,
   e.g. 01(0^14) for Nat8, 11(0^14) for Int8
   01(0^30) is used for the unit tag (the payload is a trivial zero-length bit string).
*)

module TaggingScheme = struct

  (*
     Enable for development only to sanity check value tags and
     locate unexpected tag errors to compile.ml source lines.
     Flags.sanity_check will check tags, but not further locate them.
  *)

  let debug = false (* should never be true in master! *)

  type bit = I | O
  let _ = (I,O) (* silence warning on unused constructors *)

  type _tag =
    TBool
  | TRef
  | TNum
  | TNat64 | TInt64
  | TNat32 | TInt32
  | TChar
  | TNat8 | TInt8
  | TNat16 | TInt16
  | TUnit
  | TUnused

  (* Leverage OCaml pattern match compilation to check tagging scheme is injective *)
  (* OCaml generates stack overflow for _decode:
  let _decode u64 =
    match u64 with
    | ((O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TBool (* false *)
    | ((O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,I)) -> TBool (* true *)
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,I,I)) -> TRef  (* 62 bit *)
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,I,O)) -> TNum  (* 62 bit *)
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,O,I,O,O)) -> TNat64 (* 60 bit *)
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,I,I,O,O)) -> TInt64
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (O,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TNat32
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (I,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TInt32
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (_,_,_,_,_,O,I,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TChar
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (O,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TNat16
    | ((_,_,_,_,_,_,_,_), (_,_,_,_,_,_,_,_), (I,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TInt16
    | ((_,_,_,_,_,_,_,_), (O,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TNat8
    | ((_,_,_,_,_,_,_,_), (I,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TInt8
    | ((O,I,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O), (O,O,O,O,O,O,O,O)) -> TUnit
    | _                                                                                                                                                        -> TUnused
  *)

  let tag_of_typ pty = Type.(
  if !Flags.rtti then
    match pty with
    | Nat
    | Int ->                                                                        0b10L
    | Nat64 ->                                                                    0b0100L
    | Int64 ->                                                                    0b1100L
    | Nat32 ->                                     0b01000000_00000000_00000000_00000000L
    | Int32 ->                                     0b11000000_00000000_00000000_00000000L
    | Char  ->                        0b010_00000000_00000000_00000000_00000000_00000000L
    | Nat16 ->                   0b01000000_00000000_00000000_00000000_00000000_00000000L
    | Int16 ->                   0b11000000_00000000_00000000_00000000_00000000_00000000L
    | Nat8  ->          0b01000000_00000000_00000000_00000000_00000000_00000000_00000000L
    | Int8  ->          0b11000000_00000000_00000000_00000000_00000000_00000000_00000000L
    | _  -> assert false
  else
    (* no tag *)
    match pty with
    | Nat
    | Int
    | Nat64
    | Int64
    | Nat32
    | Int32
    | Char
    | Nat16
    | Int16
    | Nat8
    | Int8 -> 0L
    | _  -> assert false)

  let unit_tag =
    if !Flags.rtti then
      (* all tag, no payload (none needed) *)
      0b01000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000L
    else
      (* no tag *)
      0L

  (* Number of payload bits in compact representation, including any sign *)
  let ubits_of pty = Type.(
    if !Flags.rtti then
      match pty with
      | Nat | Int     -> 62
      | Nat64 | Int64 -> 60
      | Nat32 | Int32 -> 32
      | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
      | Nat16 | Int16 -> 16
      | Nat8  | Int8  ->  8
      | _ -> assert false
    else
      match pty with
      | Nat   | Int   -> 63
      | Nat64 | Int64 -> 63
      | Nat32 | Int32 -> 32
      | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
      | Nat16 | Int16 -> 16
      | Nat8  | Int8  ->  8
      | _ -> assert false)

end

(*
Pointers are skewed (translated) -1 relative to the actual offset.
See documentation of module BitTagged for more detail.
*)
let ptr_skew = -1L

let ptr_unskew = 1L

module StaticBytes = struct
  (* A very simple DSL to describe static memory *)

  type t_ =
    | I32 of int32
    | I64 of int64
    | Seq of t
    | Bytes of string

  and t = t_ list

  let i64s is = Seq (List.map (fun i -> I64 i) is)

  let rec add : Buffer.t -> t_ -> unit = fun buf -> function
    | I32 i -> Buffer.add_int32_le buf i
    | I64 i -> Buffer.add_int64_le buf i
    | Seq xs -> List.iter (add buf) xs
    | Bytes b -> Buffer.add_string buf b

  let as_bytes : t -> string = fun xs ->
    let buf = Buffer.create 32 in
    List.iter (add buf) xs;
    Buffer.contents buf

  let as_words static_bytes =
    let rec convert_to_words binary index =
      assert (index <= (Bytes.length binary));
      if (Bytes.length binary) = index then
        []
      else
        let number = Bytes.get_int64_le binary index in
        let next_index = Int.add index 8 in
        [number] @ (convert_to_words binary next_index)
    in
    convert_to_words (Bytes.of_string (as_bytes static_bytes)) 0

end (* StaticBytes *)

module Const = struct

  (* Literals, as used in constant values. This is a projection of Ir.Lit,
     combining cases whose details we no longer care about.
     Should be still precise enough to map to the cases supported by SR.t.

     In other words: It is the smallest type that allows these three functions:

       (* projection of Ir.list. NB: pure, no access to env *)
       const_lit_of_lit : Ir.lit -> Const.lit (* NB: pure, no access to env *)

       (* creates vanilla representation (e.g. to put in static data structures *)
       vanilla_lit : E.env -> Const.lit -> i64

       (* creates efficient stack representation *)
       compile_lit : E.env -> Const.lit -> (SR.t, code)

  *)

  type lit =
    | Vanilla of int64 (* small words, no static data, already in vanilla format *)
    | BigInt of Big_int.big_int
    | Bool of bool
    | Word64 of Type.prim * int64
    | Float64 of Numerics.Float.t
    | Text of string
    | Blob of string
    | Null


  let tag_lit = function
    | Vanilla _ -> 0
    | BigInt _ -> 1
    | Word64 _ -> 2
    | Float64 _ -> 3
    | Bool _ -> 4
    | Text _ -> 5
    | Blob _ -> 6
    | Null -> 7

  let compare_lit l1 l2 = match l1, l2 with
    | Vanilla i, Vanilla j -> Int64.compare i j
    | BigInt i, BigInt j -> Big_int.compare_big_int i j
    | Word64 (tyi, i), Word64 (tyj, j) ->
      (match (Type.Ord.compare (Type.Prim tyi) (Type.Prim tyj)) with
       | 0 -> Int64.compare i j
       | ord -> ord)
    | Float64 i, Float64 j ->
       Int64.compare
       (Int64.bits_of_float (Mo_values.Numerics.Float.to_float i))
       (Int64.bits_of_float (Mo_values.Numerics.Float.to_float j))
    | Bool i, Bool j -> Bool.compare i j
    | Text s, Text t -> String.compare s t
    | Blob s, Blob t -> String.compare s t
    | Null, Null -> 0
    | _ -> Int.compare (tag_lit l1) (tag_lit l2)

  let lit_eq l1 l2 = compare_lit l1 l2 = 0

  (* Inlineable functions

     The prelude/prim.mo is full of functions simply wrapping a prim, e.g.

        func int64ToNat64(n : Int64) : Nat64 = (prim "num_wrap_Int64_Nat64" : Int64 -> Nat64) n;

     generating a Wasm function for them and calling them is absurdly expensive
     when the prim is just a simple Wasm instruction. Also, it requires boxing
     and unboxing arguments and results.

     So we recognize such functions when creating the `const` summary, and use the prim
     directly when calling such function.

     Can be extended to cover more forms of inlineable functions.
  *)
  type fun_rhs =
    | Complicated (* no inlining possible *)
    | PrimWrapper of Ir.prim

  (* Constant known values.

     These are values that
     * are completely known constantly
     * do not require Wasm code to be executed (e.g. in `start`)
     * can be used directly (e.g. Call, not CallIndirect)
     * can be turned into Vanilla heap data on demand

     See ir_passes/const.ml for what precisely we can compile as const now.
  *)

  type v =
    | Fun of int32 * (unit -> int32) * fun_rhs (* function pointer calculated upon first use *)
    | Message of int32 (* anonymous message, only temporary *)
    | Obj of (string * v) list
    | Unit
    | Array of v list (* immutable arrays *)
    | Tuple of v list (* non-nullary tuples *)
    | Tag of (string * v)
    | Opt of v
    | Lit of lit

  let tag = function
    | Fun _ -> 0
    | Message _ -> 1
    | Obj _ -> 2
    | Unit -> 3
    | Array _ -> 4
    | Tuple _ -> 5
    | Tag _ -> 6
    | Opt _ -> 7
    | Lit _ -> 8

  (* Ordering *)

  type t = v

  let rec compare v1 v2 = match v1, v2 with
    | Fun (id1, _, _), Fun (id2, _, _) -> Int32.compare id1 id2
    | Message fi1, Message fi2 ->  Int32.compare fi1 fi2
    | Obj fields1, Obj fields2 ->
      List.compare compare_fields fields1 fields2
    | Unit, Unit -> 0
    | Array elements1, Array elements2 ->
      List.compare compare elements1 elements2
    | Tuple elements1, Tuple elements2 ->
      List.compare compare elements1 elements2
    | Tag (name1, tag_value1), Tag (name2, tag_value2) ->
      (match String.compare name1 name2 with
        | 0 -> compare tag_value1 tag_value2
        | ord -> ord)
    | Opt opt_value1, Opt opt_value2 ->
      compare opt_value1 opt_value2
    | Lit l1, Lit l2 -> compare_lit l1 l2
    | _ -> Int.compare (tag v1) (tag v2)

  and compare_fields (name1, field_value1) (name2, field_value2) =
        match String.compare name1 name2 with
        | 0 -> compare field_value1 field_value2
        | ord -> ord

  let eq v1 v2 = compare v1 v2 = 0

end (* Const *)

module SR = struct
  (* This goes with the StackRep module, but we need the types earlier *)

  (* Value representation on the stack:

     Compiling an expression means putting its value on the stack. But
     there are various ways of putting a value onto the stack -- unboxed,
     tupled etc.
   *)
  type t =
    | Vanilla
    | UnboxedTuple of int
    | UnboxedWord64 of Type.prim
    | UnboxedFloat64
    | Unreachable
    | Const of Const.v

  let unit = UnboxedTuple 0

  let bool = Vanilla

  (* Because t contains Const.t, and that contains Const.v, and that contains
     Const.lit, and that contains Big_int, we cannot just use normal `=`. So we
     have to write our own equality.
  *)
  let eq (t1 : t) (t2 : t) = match t1, t2 with
    | Const c1, Const c2 -> Const.eq c1 c2
    | _ -> t1 = t2

  let to_var_type : t -> value_type = function
    | Vanilla -> I64Type
    | UnboxedWord64 _ -> I64Type
    | UnboxedFloat64 -> F64Type
    | UnboxedTuple n -> fatal "to_var_type: UnboxedTuple"
    | Const _ -> fatal "to_var_type: Const"
    | Unreachable -> fatal "to_var_type: Unreachable"

end (* SR *)

(*

** The compiler environment.

Of course, as we go through the code we have to track a few things; these are
put in the compiler environment, type `E.t`. Some fields are valid globally, some
only make sense locally, i.e. within a single function (but we still put them
in one big record, for convenience).

The fields fall into the following categories:

 1. Static global fields. Never change.
    Example: whether we are compiling with -no-system-api

 2. Mutable global fields. Change only monotonically.
    These are used to register things like functions. This should be monotone
    in the sense that entries are only added, and that the order should not
    matter in a significant way. In some instances, the list contains futures
    so that we can reserve and know the _position_ of the thing before we have
    to actually fill it in.

 3. Static local fields. Never change within a function.
    Example: number of parameters and return values

 4. Mutable local fields. See above
    Example: Name and type of locals.

**)
module E = struct
  include Compile_common.E

  (* The environment type *)
  module StringEnv = Env.Make(String)
  module LabSet = Set.Make(String)
  module FeatureSet = Set.Make(String)
  module ConstEnv = Env.Make(Const)

  module FunEnv = Env.Make(Int32)

  type type_descriptor = {
    candid_data_segment : int32;
    type_offsets_segment : int32;
    idl_types_segment : int32;
    }

  (* Compile-time-known value, either a plain vanilla constant or a shared object. *)
  type shared_value =
  | Vanilla of int64
  | SharedObject of int64 (* index in object pool *)

  (* Object allocation code. *)
  type object_allocation = t -> G.t
  (* Pool of shared objects.
     Alllocated in the dynamic heap on program initialization/upgrade.
     Identified by the index position in this list and accessed via the runtime system.
     Registered as GC root set and replaced on program upgrade.
  *)
  and object_pool = {
    objects: (int * object_allocation) Table.t ref;
    frozen: bool ref;
  }
  and t = {
    (* Global fields *)
    (* Static *)
    mode : Flags.compile_mode;
    rts : Wasm_exts.CustomModule.extended_module option; (* The rts. Re-used when compiling actors *)
    trap_with : t -> string -> G.t;
      (* Trap with message; in the env for dependency injection *)

    (* Per module fields (only valid/used inside a module) *)
    (* Immutable *)

    (* Mutable *)
    imports : Imports.t;
    exports : export Table.t ref;
    func_ptrs : int32 FunEnv.t ref;
    end_of_table : int32 ref;
    globals : (global Lib.Promise.t * string) Table.t ref;
    global_names : int32 NameEnv.t ref;
    built_in_funcs : lazy_function NameEnv.t ref;
    static_strings : int32 StringEnv.t ref;
    data_segments : string Table.t ref; (* Passive data segments *)

    constant_pool : shared_value ConstEnv.t ref;
    object_pool : object_pool;

    (* Types accumulated in global typtbl (for candid subtype checks)
       See Note [Candid subtype checks]
    *)
    typtbl_typs : Type.typ Table.t ref;

    (* Metadata *)
    args : (bool * string) option ref;
    service : (bool * string) option ref;
    stable_types : (bool * string) option ref;
    labs : LabSet.t ref; (* Used labels (fields and variants),
                            collected for Motoko custom section 0 *)

    (* Local fields (only valid/used inside a function) *)
    (* Static *)
    n_param : int32; (* Number of parameters (to calculate indices of locals) *)
    return_arity : int; (* Number of return values (for type of Return) *)

    (* Mutable *)
    locals : value_type Table.t ref; (* Types of locals *)
    local_names : (int32 * string) Table.t ref; (* Names of locals *)

    features : FeatureSet.t ref; (* Wasm features using wasmtime naming *)

    (* requires stable memory (and emulation on wasm targets) *)
    requires_stable_memory : bool ref;

    (* Type descriptor of current program version, created on `conclude_module`. *)
    global_type_descriptor : type_descriptor option ref;

    (* Counter for deriving a unique id per constant function. *)
    constant_functions : int32 ref;
    dedup : (unit -> int32) option ref;
    
    (* A raw HTTP handler, should skip Candid de/serialization. *)
    compiling_raw_handler : bool ref;
  }


  (* The initial global environment *)
  let mk_global mode rts trap_with : t = {
    mode;
    rts;
    trap_with;
    imports = Imports.empty ();
    exports = ref Table.empty;
    func_ptrs = ref FunEnv.empty;
    end_of_table = ref 0l;
    globals = ref Table.empty;
    global_names = ref NameEnv.empty;
    built_in_funcs = ref NameEnv.empty;
    static_strings = ref StringEnv.empty;
    data_segments = ref Table.empty;
    constant_pool = ref ConstEnv.empty;
    object_pool = { objects = ref Table.empty; frozen = ref false };
    typtbl_typs = ref Table.empty;
    (* Metadata *)
    args = ref None;
    service = ref None;
    stable_types = ref None;
    labs = ref LabSet.empty;
    (* Actually unused outside mk_fun_env: *)
    n_param = 0l;
    return_arity = 0;
    locals = ref Table.empty;
    local_names = ref Table.empty;
    features = ref FeatureSet.empty;
    requires_stable_memory = ref false;
    global_type_descriptor = ref None;
    constant_functions = ref 0l;
    dedup = ref None;
    compiling_raw_handler = ref false;
  }

  (* This wraps Mo_types.Hash.hash to also record which labels we have seen,
      so that that data can be put in a custom section, useful for debugging.
      Thus Mo_types.Hash.hash should not be called directly!
   *)
  let hash (env : t) lab =
    env.labs := LabSet.add lab !(env.labs);
    Wasm.I64_convert.extend_i32_u (Mo_types.Hash.hash lab)

  let get_labs env = LabSet.elements !(env.labs)

  let mk_fun_env env n_param return_arity =
    { env with
      n_param;
      return_arity;
      locals = ref Table.empty;
      local_names = ref Table.empty;
    }

  (* We avoid accessing the fields of t directly from outside of E, so here are a
     bunch of accessors. *)

  let mode (env : t) = env.mode

  let add_anon_local (env : t) ty =
    let i = reg env.locals ty in
    Wasm.I32.add env.n_param i

  let add_local_name (env : t) li name =
    let _ = reg env.local_names (li, name) in ()

  let get_locals (env : t) = Table.to_list !(env.locals)
  let get_local_names (env : t) : (int32 * string) list = Table.to_list !(env.local_names)

  let add_export (env : t) e =
    ignore (reg env.exports e)

  let add_global (env : t) name g =
    assert (not (NameEnv.mem name !(env.global_names)));
    let gi = reg env.globals (g, name) in
    env.global_names := NameEnv.add name gi !(env.global_names)

  let add_global64_delayed (env : t) name mut : int64 -> unit =
    let p = Lib.Promise.make () in
    add_global env name p;
    (fun init ->
      Lib.Promise.fulfill p (nr {
        gtype = GlobalType (I64Type, mut);
        value = nr (G.to_instr_list (G.i (Const (nr (Wasm_exts.Values.I64 init)))))
      })
    )
  let add_global64 (env : t) name mut init =
    add_global64_delayed env name mut init

  let get_global (env : t) name : int32 =
    match NameEnv.find_opt name !(env.global_names) with
    | Some gi -> gi
    | None -> raise (Invalid_argument (Printf.sprintf "No global named %s declared" name))

  let get_global64_lazy (env : t) name mut init : int32 =
    match NameEnv.find_opt name !(env.global_names) with
    | Some gi -> gi
    | None -> add_global64 env name mut init; get_global env name

  let export_global env name =
    add_export env (nr {
      name = Lib.Utf8.decode name;
      edesc = nr (GlobalExport (nr (get_global env name)))
    })

  let get_globals (env : t) =
    List.map (fun (g,n) -> Lib.Promise.value g) (Table.to_list !(env.globals))

  let reserve_fun (env : t) = Imports.reserve_fun env.imports

  let add_fun (env : t) = Imports.add_fun env.imports

  let make_lazy_function env name : lazy_function =
    Lib.AllocOnUse.make (fun () -> reserve_fun env name)

  let get_constant_function_id (env : t) : int32 =
    let id = !(env.constant_functions) in
    env.constant_functions := (Int32.add id 1l);
    id

  let lookup_built_in (env : t) name : lazy_function =
    match NameEnv.find_opt name !(env.built_in_funcs) with
    | None ->
      let lf = make_lazy_function env name in
      env.built_in_funcs := NameEnv.add name lf !(env.built_in_funcs);
      lf
    | Some lf -> lf

  let built_in (env : t) name : int32 =
    Lib.AllocOnUse.use (lookup_built_in env name)

  let define_built_in (env : t) name mk_fun : unit =
    Lib.AllocOnUse.def  (lookup_built_in env name) mk_fun

  let get_return_arity (env : t) = env.return_arity

  let get_exports (env : t) = Table.to_list !(env.exports)
  let get_funcs (env : t) = Imports.get_funcs env.imports

  let func_type (env : t) = Imports.func_type env.imports

  let get_types (env : t) = Imports.get_types env.imports

  let add_func_import (env : t) = Imports.add_func_import env.imports

  let call_import (env : t) = Imports.call_import env.imports

  let reuse_import (env : t) = Imports.reuse_import env.imports

  let finalize_func_imports (env : t) = Imports.finalize_func_imports env.imports

  let get_rts (env : t) = env.rts

  let as_block_type env : stack_type -> block_type = function
    | [] -> ValBlockType None
    | [t] -> ValBlockType (Some t)
    | ts -> VarBlockType (nr (func_type env (FuncType ([], ts))))


  let prepare_branch_condition =
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64))
  let if0 then_block else_block =
    prepare_branch_condition ^^
    G.if0 then_block else_block
  let if1 return_type then_block else_block =
    prepare_branch_condition ^^
    G.if1 return_type then_block else_block

  let if_ env tys thn els = prepare_branch_condition ^^ G.if_ (as_block_type env tys) thn els
  let block_ env tys bdy = G.block_ (as_block_type env tys) bdy


  let trap_with env msg = env.trap_with env msg
  let then_trap_with env msg = if0 (trap_with env msg) G.nop
  let else_trap_with env msg = if0 G.nop (trap_with env msg)

  let add_data_segment (env : t) data : int32 =
    let index, t = Table.add !(env.data_segments) data in
    env.data_segments := t;
    Int32.of_int index

  let add_fun_ptr (env : t) fi : int32 =
    match FunEnv.find_opt fi !(env.func_ptrs) with
    | Some fp -> fp
    | None ->
      let fp = !(env.end_of_table) in
      env.func_ptrs := FunEnv.add fi fp !(env.func_ptrs);
      env.end_of_table := Int32.add !(env.end_of_table) 1l;
      fp

  let get_elems env =
    FunEnv.bindings !(env.func_ptrs)

  let get_end_of_table env : int32 =
    !(env.end_of_table)

  let add_static (env : t) (data : StaticBytes.t) : int32 =
    let b = StaticBytes.as_bytes data in
    match StringEnv.find_opt b !(env.static_strings)  with
    | Some segment_index -> segment_index
    | None ->
      let segment_index = add_data_segment env b  in
      env.static_strings := StringEnv.add b segment_index !(env.static_strings);
      segment_index

  let replace_data_segment (env : t) (segment_index : int32) (data : StaticBytes.t) : int64 =
    let new_value = StaticBytes.as_bytes data in
    let segment_index = Int32.to_int segment_index in
    assert (segment_index < Table.length !(env.data_segments));
    env.data_segments := Table.from_list (
      List.mapi (fun index old_value ->
      if index = segment_index then
        (assert (old_value = "");
        new_value)
      else
        old_value
      ) (Table.to_list !(env.data_segments)));
    Int64.of_int (String.length new_value)

  let get_data_segments (env : t) =
    Table.to_list !(env.data_segments)

  let constant_pool_add env const make_shared_value =
    match ConstEnv.find_opt const !(env.constant_pool) with
    | Some shared_value -> shared_value
    | None ->
      let sv = make_shared_value() in
      match sv with
      | SharedObject _ ->
        env.constant_pool := ConstEnv.add const sv !(env.constant_pool);
        sv
      | Vanilla _ -> sv

  let object_pool_add (env : t) line (allocation : t -> G.t) : int64 =
    if !(env.object_pool.frozen) then raise (Invalid_argument "Object pool frozen");
    let index, t = Table.add !(env.object_pool.objects) (line, allocation) in
    env.object_pool.objects := t;
    Int64.of_int index

  let object_pool_size (env : t) : int =
    Table.length !(env.object_pool.objects)

  let object_pool_report (env : t) : unit =
    let e = ref StringEnv.empty in
    List.iter (fun (l, _) ->
        let line = Int.to_string l in
        e := StringEnv.add line
               (match StringEnv.find_opt line (!e) with
                | None -> 1
                | Some i -> i + 1) (!e))
    (Table.to_list !(env.object_pool.objects));
    let profile = StringEnv.fold (fun l c s -> s ^ "\n" ^ __FILE__^ ", line " ^ l ^ "[" ^ Int.to_string c ^ "]") (!e) ""
    in
    begin
      Printf.eprintf "\ndata segments = %i" (Table.length !(env.data_segments));
      Printf.eprintf "\nshared constants = %i" (ConstEnv.cardinal !(env.constant_pool));
      Printf.eprintf "\npool size = %i" (object_pool_size env);
      Printf.eprintf "\npool report = %s" profile
    end

  let iterate_object_pool (env : t) f =
    G.concat_mapi f (List.map (fun (l, a) -> a) (Table.to_list !(env.object_pool.objects)))

  let collect_garbage env force =
    let name = "incremental_gc" in
    let gc_fn = if force || !Flags.force_gc then name else "schedule_" ^ name in
    call_import env "rts" gc_fn

  (* See Note [Candid subtype checks] *)
  (* NB: we don't bother detecting duplicate registrations here because the code sharing machinery
     ensures that `add_typtbl_typ t` is called at most once for any `t` with a distinct type hash *)
  let add_typtbl_typ (env : t) ty : Int32.t =
    reg env.typtbl_typs ty

  let get_typtbl_typs (env : t) : Type.typ list =
    Table.to_list !(env.typtbl_typs)

  let add_feature (env : t) f =
    env.features := FeatureSet.add f !(env.features)

  let get_features (env : t) = FeatureSet.elements !(env.features)

  let require_stable_memory (env : t)  =
    if not !(env.requires_stable_memory)
    then
      (env.requires_stable_memory := true;
       match mode env with
       | Flags.ICMode | Flags.RefMode ->
          ()
       | Flags.WASIMode | Flags.WasmMode ->
          add_feature env "bulk-memory";
          add_feature env "multi-memory")

  let requires_stable_memory (env : t) =
    !(env.requires_stable_memory)

  let get_memories (env : t) initial_memory_pages =
    nr {mtype = MemoryType ({min = initial_memory_pages; max = None}, I64IndexType)}
    ::
    match mode env with
    | Flags.WASIMode | Flags.WasmMode when !(env.requires_stable_memory) ->
      [ nr {mtype = MemoryType ({min = Int64.zero; max = None}, I64IndexType)} ]
    | _ -> []

  let get_dedup (env : t) : int32 =
    match !(env.dedup) with
    | Some mk_fi -> mk_fi()
    | None -> assert false

  let set_dedup (env : t) (mk_fi : unit -> int32)=
    env.dedup := Some mk_fi

end


(* General code generation functions:
   Rule of thumb: Here goes stuff that independent of the Motoko AST.
*)

(* Function called compile_* return a list of instructions (and maybe other stuff) *)


let compile_comparison rel =
  G.i (Compare (Wasm_exts.Values.I64 rel)) ^^
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
let compile_comparison_32 rel =
  G.i (Compare (Wasm_exts.Values.I32 rel)) ^^
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
let compile_test op =
  G.i (Test (Wasm_exts.Values.I64 op)) ^^
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
let compile_comparison_f64 rel =
  G.i (Compare (Wasm_exts.Values.F64 rel)) ^^
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))

let compile_unboxed_const i = G.i (Const (nr (Wasm_exts.Values.I64 i)))
let compile_const_32 i = G.i (Const (nr (Wasm_exts.Values.I32 i)))
let compile_unboxed_zero = compile_unboxed_const 0L
let compile_unboxed_one = compile_unboxed_const 1L

(* Some common arithmetic, used for pointer and index arithmetic *)
let compile_op_const op i =
    compile_unboxed_const i ^^
    G.i (Binary (Wasm_exts.Values.I64 op))
let compile_add_const = compile_op_const I64Op.Add
let compile_sub_const = compile_op_const I64Op.Sub
let compile_divU_const = compile_op_const I64Op.DivU
let compile_shrU_const = compile_op_const I64Op.ShrU
let compile_shrS_const = compile_op_const I64Op.ShrS
let compile_shl_const = compile_op_const I64Op.Shl
let compile_mul_const = function
  | 0L -> G.i Drop ^^ compile_unboxed_zero
  | 1L -> G.nop
  | n when n > 0L && Numerics.Nat64.(of_int64 n |> popcnt |> to_int64) = 1L
    -> compile_shl_const Numerics.Nat64.(of_int64 n |> ctz |> to_int64)
  | n -> compile_op_const I64Op.Mul n
let compile_rotl_const = compile_op_const I64Op.Rotl
let compile_rotr_const = compile_op_const I64Op.Rotr
let compile_bitand_const = compile_op_const I64Op.And
let compile_bitor_const = function
  | 0L -> G.nop | n -> compile_op_const I64Op.Or n
let compile_xor_const = function
  | 0L -> G.nop | n -> compile_op_const I64Op.Xor n
let compile_rel_const rel i =
  compile_unboxed_const i ^^
  compile_comparison rel
let compile_eq_const = function
  | 0L -> compile_test I64Op.Eqz
  | i -> compile_rel_const I64Op.Eq i

let compile_op32_const op i =
    compile_const_32 i ^^
    G.i (Binary (Wasm_exts.Values.I32 op))
let compile_add32_const = compile_op32_const I32Op.Add
let _compile_sub32_const = compile_op32_const I32Op.Sub
let _compile_mul32_const = compile_op32_const I32Op.Mul
let _compile_divU32_const = compile_op32_const I32Op.DivU
let _compile_shrU32_const = function
  | 0l -> G.nop | n -> compile_op32_const I32Op.ShrU n
let _compile_shrS32_const = function
  | 0l -> G.nop | n -> compile_op32_const I32Op.ShrS n
let _compile_shl32_const = function
  | 0l -> G.nop | n -> compile_op32_const I32Op.Shl n
let compile_eq32_const i =
  compile_const_32 i ^^
  compile_comparison_32 I32Op.Eq

(* Analogous to Lib.Uint32.compare *)
let compare_uint64 i1 i2 =
  if i1 < 0L && i2 >= 0L then 1
  else if i1 >= 0L && i2 < 0L then -1
  else Int64.compare i1 i2

(* A common variant of todo *)

let todo_trap env fn se = todo fn se (E.trap_with env ("TODO: " ^ fn))
let _todo_trap_SR env fn se = todo fn se (SR.Unreachable, E.trap_with env ("TODO: " ^ fn))

(* Locals *)

let new_local_ env t name =
  let i = E.add_anon_local env t in
  E.add_local_name env i name;
  ( G.i (LocalSet (nr i))
  , G.i (LocalGet (nr i))
  , i
  )

let new_local env name =
  let (set_i, get_i, _) = new_local_ env I64Type name
  in (set_i, get_i)

let new_local32 env name =
  let (set_i, get_i, _) = new_local_ env I32Type name
  in (set_i, get_i)

(* Some common code macros *)

(* Iterates while cond is true. *)
let compile_while env cond body =
    G.loop0 (
      cond ^^ E.if0 (body ^^ G.i (Br (nr 1l))) G.nop
    )

(* Expects a number n on the stack. Iterates from m to below that number. *)
let from_m_to_n env m mk_body =
    let (set_n, get_n) = new_local env "n" in
    let (set_i, get_i) = new_local env "i" in
    set_n ^^
    compile_unboxed_const m ^^
    set_i ^^

    compile_while env
      ( get_i ^^
        get_n ^^
        compile_comparison I64Op.LtU
      ) (
        mk_body get_i ^^

        get_i ^^
        compile_add_const 1L ^^
        set_i
      )

(* Expects a number on the stack. Iterates from zero to below that number. *)
let from_0_to_n env mk_body = from_m_to_n env 0L mk_body

(* Pointer reference and dereference  *)

let load_unskewed_ptr : G.t =
  G.i (Load {ty = I64Type; align = 3; offset = 0L; sz = None})

let store_unskewed_ptr : G.t =
  G.i (Store {ty = I64Type; align = 3; offset = 0L; sz = None})

let load_ptr : G.t =
  G.i (Load {ty = I64Type; align = 3; offset = ptr_unskew; sz = None})

let store_ptr : G.t =
  G.i (Store {ty = I64Type; align = 3; offset = ptr_unskew; sz = None})

let narrow_to_32 env get_value =
  get_value ^^
  compile_unboxed_const 0xffff_ffffL ^^
  compile_comparison I64Op.LeU ^^
  E.else_trap_with env "cannot narrow to 32 bit" ^^ (* Note: If narrow fails during print, the trap print leads to an infinite recursion and a stack overflow *)
  get_value ^^
  G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64))

module FakeMultiVal = struct
  (* For some use-cases (e.g. processing the compiler output with analysis
     tools) it is useful to avoid the multi-value extension.

     This module provides mostly transparent wrappers that put multiple values
     in statically allocated globals and pull them off again.

     So far only does I64Type (but that could be changed).

     If the multi_value flag is on, these do not do anything.
  *)
  let ty tys =
    if !Flags.multi_value || List.length tys <= 1
    then tys
    else []

  let global env i =
    E.get_global64_lazy env (Printf.sprintf "multi_val_%d" i) Mutable 0L

  let store env tys =
    if !Flags.multi_value || List.length tys <= 1 then G.nop else
    G.concat_mapi (fun i ty ->
      assert(ty = I64Type);
      G.i (GlobalSet (nr (global env i)))
    ) tys

  let load env tys =
    if !Flags.multi_value || List.length tys <= 1 then G.nop else
    let n = List.length tys - 1 in
    G.concat_mapi (fun i ty ->
      assert(ty = I64Type);
      G.i (GlobalGet (nr (global env (n - i))))
    ) tys

  (* A drop-in replacement for E.if_ *)
  let if_ env bt thn els =
    E.if_ env (ty bt) (thn ^^ store env bt) (els ^^ store env bt) ^^
    load env bt

  (* A block that can be exited from *)
  let block_ env bt body =
    E.block_ env (ty bt) (G.with_current_depth (fun depth ->
      body (store env bt ^^ G.branch_to_ depth)
    )) ^^
    load env bt

end (* FakeMultiVal *)

module Func = struct
  (* This module contains basic bookkeeping functionality to define functions,
     in particular creating the environment, and finally adding it to the environment.
  *)


  let of_body env params retty mk_body =
    let env1 = E.mk_fun_env env (Int32.of_int (List.length params)) (List.length retty) in
    List.iteri (fun i (n,_t) -> E.add_local_name env1 (Int32.of_int i) n) params;
    let ty = FuncType (List.map snd params, FakeMultiVal.ty retty) in
    let body = G.to_instr_list (
      mk_body env1 ^^ FakeMultiVal.store env1 retty
    ) in
    (nr { ftype = nr (E.func_type env ty);
          locals = E.get_locals env1;
          body }
    , E.get_local_names env1)

  let define_built_in env name params retty mk_body =
    E.define_built_in env name (lazy (of_body env params retty mk_body))

  type sharing =
    Always (* i.e. never inline *)
  | Never  (* i.e. always inline *)

  (* (Almost) transparently lift code into a function and call this function,
     unless sharing = Never and not (!Flags.share_code) in which case the code
     is inlined.
     NB: inlined code must not be recursive nor `return`.
  *)
  (* Also add a hack to support multiple return values *)
  let share_code sharing env name params retty mk_body =
    if sharing = Always || !Flags.share_code
    then
      let getters =
        List.mapi
          (fun i (n, t) -> (G.i (LocalGet (nr (Int32.of_int i)))))
          params
      in
      define_built_in env name params retty (fun env -> mk_body env getters);
      G.i (Call (nr (E.built_in env name))) ^^
      FakeMultiVal.load env retty
    else begin
      assert (sharing = Never);
      let locals =
        List.map
           (fun (n, t) -> new_local_ env t n)
           params
      in
      let set_locals = List.fold_right (fun (set, get, _) is-> is ^^ set) locals G.nop in
      let getters = List.map (fun (set, get, _) -> get) locals in
      set_locals ^^
      mk_body env getters ^^ FakeMultiVal.store env retty ^^
      FakeMultiVal.load env retty
   end

  (* Shorthands for various arities *)
  let [@warning "-8"] share_code0 sharing env name retty mk_body =
    share_code sharing env name [] retty (fun env [] -> mk_body env)
  let [@warning "-8"] share_code1 sharing env name p1 retty mk_body =
    share_code sharing env name [p1] retty (fun env [g1] -> mk_body env
        g1
    )
  let [@warning "-8"] share_code2 sharing env name (p1,p2) retty mk_body =
    share_code sharing env name [p1; p2] retty (fun env [g1; g2] -> mk_body env
      g1
      g2
    )
  let [@warning "-8"] share_code3 sharing env name (p1, p2, p3) retty mk_body =
    share_code sharing env name [p1; p2; p3] retty (fun env [g1; g2; g3] -> mk_body env
      g1
      g2
      g3
    )
  let [@warning "-8"] _share_code4 sharing env name (p1, p2, p3, p4) retty mk_body =
    share_code sharing env name [p1; p2; p3; p4] retty (fun env [g1; g2; g3; g4]-> mk_body env
      g1
      g2
      g3
      g4
    )
  let [@warning "-8"] _share_code6 sharing env name (p1, p2, p3, p4, p5, p6) retty mk_body =
    share_code sharing env name [p1; p2; p3; p4; p5; p6] retty (fun env [g1; g2; g3; g4; g5; g6] -> mk_body env
      g1
      g2
      g3
      g4
      g5
      g6
    )
  let [@warning "-8"] _share_code7 sharing env name (p1, p2, p3, p4, p5, p6, p7) retty mk_body =
    share_code sharing env name [p1; p2; p3; p4; p5; p6; p7] retty (fun env [g1; g2; g3; g4; g5; g6; g7] -> mk_body env
      g1
      g2
      g3
      g4
      g5
      g6
      g7
    )

  let [@warning "-8"] _share_code9 sharing env name (p1, p2, p3, p4, p5, p6, p7, p8, p9) retty mk_body =
    share_code sharing env name [p1; p2; p3; p4; p5; p6; p7; p8; p9] retty (fun env [g1; g2; g3; g4; g5; g6; g7; g8; g9] -> mk_body env
      g1
      g2
      g3
      g4
      g5
      g6
      g7
      g8
      g9
    )


end (* Func *)

module RTS = struct
  (* The connection to the C and Rust parts of the RTS *)
  let system_imports env =
    E.add_func_import env "rts" "initialize_incremental_gc" [] [];
    E.add_func_import env "rts" "schedule_incremental_gc" [] [];
    E.add_func_import env "rts" "incremental_gc" [] [];
    E.add_func_import env "rts" "write_with_barrier" [I64Type; I64Type] [];
    E.add_func_import env "rts" "allocation_barrier" [I64Type] [I64Type];
    E.add_func_import env "rts" "running_gc" [] [I32Type];
    E.add_func_import env "rts" "register_stable_type" [I64Type; I64Type] [];
    E.add_func_import env "rts" "assign_stable_type" [I64Type; I64Type] [];
    E.add_func_import env "rts" "has_stable_actor" [] [I32Type];
    E.add_func_import env "rts" "load_stable_actor" [] [I64Type];
    E.add_func_import env "rts" "save_stable_actor" [I64Type] [];
    E.add_func_import env "rts" "free_stable_actor" [] [];
    E.add_func_import env "rts" "contains_field" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "initialize_static_variables" [I64Type] [];
    E.add_func_import env "rts" "get_static_variable" [I64Type] [I64Type];
    E.add_func_import env "rts" "set_static_variable" [I64Type; I64Type] [];
    E.add_func_import env "rts" "set_upgrade_instructions" [I64Type] [];
    E.add_func_import env "rts" "get_upgrade_instructions" [] [I64Type];
    E.add_func_import env "rts" "memcmp" [I64Type; I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "version" [] [I64Type];
    E.add_func_import env "rts" "parse_idl_header" [I32Type; I64Type; I64Type; I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "idl_alloc_typtbl" [I64Type; I64Type; I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "idl_sub_buf_words" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "idl_sub_buf_init" [I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "idl_sub"
      [I64Type; I64Type; I64Type; I64Type; I64Type; I64Type; I64Type; I32Type; I32Type] [I32Type];
    E.add_func_import env "rts" "leb128_decode" [I64Type] [I64Type];
    E.add_func_import env "rts" "sleb128_decode" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_to_word32_wrap" [I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_of_word64" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_of_int64" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_of_float64" [F64Type] [I64Type];
    E.add_func_import env "rts" "bigint_to_float64" [I64Type] [F64Type];
    E.add_func_import env "rts" "bigint_to_word64_wrap" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_to_word64_trap" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_to_word64_trap_with" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_eq" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_isneg" [I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_count_bits" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_2complement_bits" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_lt" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_gt" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_le" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_ge" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "bigint_add" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_sub" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_mul" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_rem" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_div" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_pow" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_neg" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_lsh" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_rsh" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_abs" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_leb128_size" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_leb128_encode" [I64Type; I64Type] [];
    E.add_func_import env "rts" "bigint_leb128_decode" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_leb128_decode_word64" [I64Type; I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_sleb128_size" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_sleb128_encode" [I64Type; I64Type] [];
    E.add_func_import env "rts" "bigint_sleb128_decode" [I64Type] [I64Type];
    E.add_func_import env "rts" "bigint_sleb128_decode_word64" [I64Type; I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "leb128_encode" [I64Type; I64Type] [];
    E.add_func_import env "rts" "sleb128_encode" [I64Type; I64Type] [];
    E.add_func_import env "rts" "utf8_valid" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "utf8_validate" [I64Type; I64Type] [];
    E.add_func_import env "rts" "skip_leb128" [I64Type] [];
    E.add_func_import env "rts" "skip_any" [I64Type; I64Type; I32Type; I32Type] [];
    E.add_func_import env "rts" "find_field" [I64Type; I64Type; I64Type; I32Type; I64Type] [I32Type];
    E.add_func_import env "rts" "skip_fields" [I64Type; I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "remember_continuation" [I64Type] [I64Type];
    E.add_func_import env "rts" "recall_continuation" [I64Type] [I64Type];
    E.add_func_import env "rts" "peek_future_continuation" [I64Type] [I64Type];
    E.add_func_import env "rts" "continuation_count" [] [I64Type];
    E.add_func_import env "rts" "continuation_table_size" [] [I64Type];
    E.add_func_import env "rts" "blob_of_text" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_compare" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "text_concat" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "text_iter_done" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_iter" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_iter_next" [I64Type] [I32Type];
    E.add_func_import env "rts" "text_len" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_of_ptr_size" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "text_singleton" [I32Type] [I64Type];
    E.add_func_import env "rts" "text_size" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_to_buf" [I64Type; I64Type] [];
    E.add_func_import env "rts" "text_lowercase" [I64Type] [I64Type];
    E.add_func_import env "rts" "text_uppercase" [I64Type] [I64Type];
    E.add_func_import env "rts" "region_init" [I64Type] [];
    E.add_func_import env "rts" "alloc_region" [I64Type; I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "init_region" [I64Type; I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "region_new" [] [I64Type];
    E.add_func_import env "rts" "region_id" [I64Type] [I64Type];
    E.add_func_import env "rts" "region_page_count" [I64Type] [I64Type];
    E.add_func_import env "rts" "region_vec_pages" [I64Type] [I64Type];
    E.add_func_import env "rts" "region_size" [I64Type] [I64Type];
    E.add_func_import env "rts" "region_grow" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "region_load_blob" [I64Type; I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "region_store_blob" [I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "region_load_word8" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "region_store_word8" [I64Type; I64Type; I32Type] [];
    E.add_func_import env "rts" "region_load_word16" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "region_store_word16" [I64Type; I64Type; I32Type] [];
    E.add_func_import env "rts" "region_load_word32" [I64Type; I64Type] [I32Type];
    E.add_func_import env "rts" "region_store_word32" [I64Type; I64Type; I32Type] [];
    E.add_func_import env "rts" "region_load_word64" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "region_store_word64" [I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "region_load_float64" [I64Type; I64Type] [F64Type];
    E.add_func_import env "rts" "region_store_float64" [I64Type; I64Type; F64Type] [];
    E.add_func_import env "rts" "region0_get" [] [I64Type];
    E.add_func_import env "rts" "blob_of_principal" [I64Type] [I64Type];
    E.add_func_import env "rts" "principal_of_blob" [I64Type] [I64Type];
    E.add_func_import env "rts" "compute_crc32" [I64Type] [I32Type];
    E.add_func_import env "rts" "blob_iter_done" [I64Type] [I64Type];
    E.add_func_import env "rts" "blob_iter" [I64Type] [I64Type];
    E.add_func_import env "rts" "blob_iter_next" [I64Type] [I64Type];
    E.add_func_import env "rts" "pow" [F64Type; F64Type] [F64Type];
    E.add_func_import env "rts" "sin" [F64Type] [F64Type];
    E.add_func_import env "rts" "cos" [F64Type] [F64Type];
    E.add_func_import env "rts" "tan" [F64Type] [F64Type];
    E.add_func_import env "rts" "asin" [F64Type] [F64Type];
    E.add_func_import env "rts" "acos" [F64Type] [F64Type];
    E.add_func_import env "rts" "atan" [F64Type] [F64Type];
    E.add_func_import env "rts" "atan2" [F64Type; F64Type] [F64Type];
    E.add_func_import env "rts" "exp" [F64Type] [F64Type];
    E.add_func_import env "rts" "log" [F64Type] [F64Type];
    E.add_func_import env "rts" "fmod" [F64Type; F64Type] [F64Type];
    E.add_func_import env "rts" "float_fmt" [F64Type; I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "char_to_upper" [I32Type] [I32Type];
    E.add_func_import env "rts" "char_to_lower" [I32Type] [I32Type];
    E.add_func_import env "rts" "char_is_whitespace" [I32Type] [I32Type];
    E.add_func_import env "rts" "char_is_lowercase" [I32Type] [I32Type];
    E.add_func_import env "rts" "char_is_uppercase" [I32Type] [I32Type];
    E.add_func_import env "rts" "char_is_alphabetic" [I32Type] [I32Type];
    E.add_func_import env "rts" "get_max_live_size" [] [I64Type];
    E.add_func_import env "rts" "get_reclaimed" [] [I64Type];
    E.add_func_import env "rts" "alloc_words" [I64Type] [I64Type];
    E.add_func_import env "rts" "get_total_allocations" [] [I64Type];
    E.add_func_import env "rts" "get_heap_size" [] [I64Type];
    E.add_func_import env "rts" "alloc_blob" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "alloc_array" [I64Type; I64Type] [I64Type];
    E.add_func_import env "rts" "read_persistence_version" [] [I64Type];
    E.add_func_import env "rts" "stop_gc_before_stabilization" [] [];
    E.add_func_import env "rts" "start_gc_after_destabilization" [] [];
    E.add_func_import env "rts" "is_graph_stabilization_started" [] [I32Type];
    E.add_func_import env "rts" "start_graph_stabilization" [I64Type; I64Type; I64Type] [];
    E.add_func_import env "rts" "graph_stabilization_increment" [] [I32Type];
    E.add_func_import env "rts" "start_graph_destabilization" [I64Type; I64Type] [];
    E.add_func_import env "rts" "graph_destabilization_increment" [] [I32Type];
    E.add_func_import env "rts" "get_graph_destabilized_actor" [] [I64Type];
    E.add_func_import env "rts" "buffer_in_32_bit_range" [] [I64Type];
    E.add_func_import env "rts" "alloc_weak_ref" [I64Type] [I64Type];
    E.add_func_import env "rts" "weak_ref_is_live" [I64Type] [I32Type];
    E.add_func_import env "rts" "get_dedup_table" [] [I64Type];
    E.add_func_import env "rts" "set_dedup_table" [I64Type] [];
    ()

end (* RTS *)

module GC = struct
  (* Record mutator/gc instructions counts *)

  let instruction_counter env =
    compile_const_32 0l ^^
    E.call_import env "ic0" "performance_counter"

  let register_globals env =
    E.add_global64 env "__mutator_instructions" Mutable 0L;
    E.add_global64 env "__collector_instructions" Mutable 0L;
    E.add_global64 env "__lifetime_instructions" Mutable 0L

  let get_mutator_instructions env =
    G.i (GlobalGet (nr (E.get_global env "__mutator_instructions")))
  let set_mutator_instructions env =
    G.i (GlobalSet (nr (E.get_global env "__mutator_instructions")))

  let get_collector_instructions env =
    G.i (GlobalGet (nr (E.get_global env "__collector_instructions")))
  let set_collector_instructions env =
    G.i (GlobalSet (nr (E.get_global env "__collector_instructions")))

  let get_lifetime_instructions env =
    G.i (GlobalGet (nr (E.get_global env "__lifetime_instructions")))
  let set_lifetime_instructions env =
    G.i (GlobalSet (nr (E.get_global env "__lifetime_instructions")))

  let record_mutator_instructions env =
    match E.mode env with
    | Flags.(ICMode | RefMode)  ->
      instruction_counter env ^^
      set_mutator_instructions env
    | _ -> G.nop

  let record_collector_instructions env =
    match E.mode env with
    | Flags.(ICMode | RefMode)  ->
      instruction_counter env ^^
      get_mutator_instructions env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
      set_collector_instructions env
    | _ -> G.nop

  let record_lifetime_instructions env =
    match E.mode env with
    | Flags.(ICMode | RefMode)  ->
      get_mutator_instructions env ^^
      get_lifetime_instructions env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
      get_collector_instructions env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
      set_lifetime_instructions env
    | _ -> G.nop

  let collect_garbage env =
    record_mutator_instructions env ^^
    E.collect_garbage env false ^^
    record_collector_instructions env ^^
    record_lifetime_instructions env

end (* GC *)

module Heap = struct
  (* General heap object functionality (allocation, setting fields, reading fields) *)

  (* Memory addresses are 64 bit (I64Type). *)
  let word_size = 8L


  (* The heap base global can only be used late, see conclude_module
     and GHC.register *)
  let get_heap_base env =
    G.i (GlobalGet (nr (E.get_global env "__heap_base")))

  let get_total_allocation env =
    E.call_import env "rts" "get_total_allocations"

  let get_reclaimed env =
    E.call_import env "rts" "get_reclaimed"

  let get_memory_size =
    G.i MemorySize ^^
    compile_mul_const page_size

  let get_max_live_size env =
    E.call_import env "rts" "get_max_live_size"

  (* Static allocation (always words)
     (uses dynamic allocation for smaller and more readable code) *)
  let alloc env (n : int64) : G.t =
    compile_unboxed_const n ^^
    E.call_import env "rts" "alloc_words"

  (* Heap objects *)

  (* At this level of abstraction, heap objects are just flat arrays of words *)

  let load_field (i : int64) : G.t =
    let offset = Int64.(add (mul word_size i) ptr_unskew) in
    G.i (Load {ty = I64Type; align = 3; offset; sz = None})

  let store_field (i : int64) : G.t =
    let offset = Int64.(add (mul word_size i) ptr_unskew) in
    G.i (Store {ty = I64Type; align = 3; offset; sz = None})

  (* Or even as a single 64 bit float *)

  let load_field_float64 (i : int64) : G.t =
    let offset = Int64.(add (mul word_size i) ptr_unskew) in
    G.i (Load {ty = F64Type; align = 3; offset; sz = None})

  let store_field_float64 (i : int64) : G.t =
    let offset = Int64.(add (mul word_size i) ptr_unskew) in
    G.i (Store {ty = F64Type; align = 3; offset; sz = None})

  (* Convenience functions related to memory *)
  (* Copying bytes (works on unskewed memory addresses) *)
  let memcpy env = G.i MemoryCopy
  (* Comparing bytes (works on unskewed memory addresses) *)
  let memcmp env = E.call_import env "rts" "memcmp" ^^ G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))

  let register env =
    let get_heap_base_fn = E.add_fun env "get_heap_base" (Func.of_body env [] [I64Type] (fun env ->
      get_heap_base env
    )) in

    E.add_export env (nr {
      name = Lib.Utf8.decode "get_heap_base";
      edesc = nr (FuncExport (nr get_heap_base_fn))
    })

  let get_heap_size env =
    E.call_import env "rts" "get_heap_size"

  let get_static_variable env index =
    compile_unboxed_const index ^^
    E.call_import env "rts" "get_static_variable"

end (* Heap *)

module Stack = struct
  (* The RTS includes C code which requires a shadow stack in linear memory.
     We reserve some space for it at the beginning of memory space (just like
     wasm-l would), this way stack overflow would cause out-of-memory, and not
     just overwrite static data.

     We sometimes use the stack space if we need small amounts of scratch space.

     All pointers here are unskewed.

     (We report logical stack overflow as "RTS Stack underflow" as the stack
     grows downwards.)
  *)

  (* Predefined constant stack size of 4MB, according to the persistent memory layout. *)
  let stack_size = 4 * 1024 * 1024

  let end_ () = Int64.of_int stack_size

  let register_globals env =
    (* stack pointer *)
    E.add_global64 env "__stack_pointer" Mutable (end_());
    (* frame pointer *)
    E.add_global64 env "__frame_pointer" Mutable (end_());
    (* low watermark *)
    if !Flags.measure_rts_stack then
      E.add_global64 env "__stack_min" Mutable (end_());
    E.export_global env "__stack_pointer"

  let get_stack_ptr env =
    G.i (GlobalGet (nr (E.get_global env "__stack_pointer")))
  let set_stack_ptr env =
    G.i (GlobalSet (nr (E.get_global env "__stack_pointer")))

  let get_min env =
    G.i (GlobalGet (nr (E.get_global env "__stack_min")))
  let set_min env =
    G.i (GlobalSet (nr (E.get_global env "__stack_min")))

  let get_max_stack_size env =
    if !Flags.measure_rts_stack then
      compile_unboxed_const (end_()) ^^
      get_min env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
    else (* report max available *)
      compile_unboxed_const (end_())

  let update_stack_min env =
    if !Flags.measure_rts_stack then
    get_stack_ptr env ^^
    get_min env ^^
    compile_comparison I64Op.LtU ^^
    (E.if0
       (get_stack_ptr env ^^
        set_min env)
      G.nop)
    else G.nop

  let stack_overflow env =
    Func.share_code0 Func.Never env "stack_overflow" [] (fun env ->
      (* read last word of reserved page to force trap *)
      compile_unboxed_const 0xFFFF_FFFF_FFFF_FFFCL ^^
      G.i (Load {ty = I64Type; align = 3; offset = 0L; sz = None}) ^^
      G.i Unreachable
    )

  let alloc_words env n =
    let n_bytes = Int64.mul n Heap.word_size in
    (* avoid absurd allocations *)
    assert (Int64.(to_int n_bytes) < stack_size);
    (* alloc words *)
    get_stack_ptr env ^^
    compile_unboxed_const n_bytes ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
    set_stack_ptr env ^^
    update_stack_min env ^^
    get_stack_ptr env ^^
    (* check for stack overflow, if necessary *)
    if n_bytes >= page_size then
      get_stack_ptr env ^^
      G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
      E.if0
        G.nop (* we found leading zeros, i.e. no wraparound *)
        (stack_overflow env)
    else
      G.nop

  let free_words env n =
    get_stack_ptr env ^^
    compile_unboxed_const (Int64.mul n Heap.word_size) ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    set_stack_ptr env

  (* TODO: why not just remember and reset the stack pointer, instead of calling free_words? Also below *)
  let with_words env name n f =
    let (set_x, get_x) = new_local env name in
    alloc_words env n ^^ set_x ^^
    f get_x ^^
    free_words env n

  let dynamic_alloc_words env get_n =
    get_stack_ptr env ^^
    compile_divU_const Heap.word_size ^^
    get_n ^^
    compile_comparison I64Op.LtU ^^
    (E.if0
      (stack_overflow env)
      G.nop) ^^
    get_stack_ptr env ^^
    get_n ^^
    compile_mul_const Heap.word_size ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
    set_stack_ptr env ^^
    update_stack_min env ^^
    get_stack_ptr env

  let dynamic_free_words env get_n =
    get_stack_ptr env ^^
    get_n ^^
    compile_mul_const Heap.word_size ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    set_stack_ptr env

  (* TODO: why not just remember and reset the stack pointer, instead of calling free_words? Also above*)
  let dynamic_with_words env name f =
    let (set_n, get_n) = new_local env "n" in
    let (set_x, get_x) = new_local env name in
    set_n ^^
    dynamic_alloc_words env get_n ^^ set_x ^^
    f get_x ^^
    dynamic_free_words env get_n

  let dynamic_with_bytes env name f =
    (* round up to nearest wordsize *)
    compile_add_const (Int64.sub Heap.word_size 1L) ^^
    compile_divU_const Heap.word_size ^^
    dynamic_with_words env name f

  (* Stack Frames *)

  (* Traditional frame pointer for accessing statically allocated locals/args (all words)
     Used (sofar) only in serialization to compress Wasm stack
     at cost of expanding Rust/C Stack (whose size we control)*)
  let get_frame_ptr env =
    G.i (GlobalGet (nr (E.get_global env "__frame_pointer")))
  let set_frame_ptr env =
    G.i (GlobalSet (nr (E.get_global env "__frame_pointer")))

  (* Frame pointer operations *)

  (* Enter/exit a new frame of `n` words, saving and restoring prev frame pointer *)
  let with_frame env name n f =
    (* reserve space for n words + saved frame_ptr *)
    alloc_words env (Int64.add n 1L) ^^
    (* store the current frame_ptr at offset 0 *)
    get_frame_ptr env ^^
    G.i (Store {ty = I64Type; align = 3; offset = 0L; sz = None}) ^^
    get_stack_ptr env ^^
    (* set_frame_ptr to stack_ptr *)
    set_frame_ptr env ^^
    (* do as f *)
    f () ^^
    (* assert frame_ptr == stack_ptr *)
    get_frame_ptr env ^^
    get_stack_ptr env ^^
    compile_comparison I64Op.Eq ^^
    E.else_trap_with env "frame_ptr <> stack_ptr" ^^
    (* restore the saved frame_ptr *)
    get_frame_ptr env ^^
    G.i (Load {ty = I64Type; align = 3; offset = 0L; sz = None}) ^^
    set_frame_ptr env ^^
    (* free the frame *)
    free_words env (Int64.add n 1L)

  (* read local n of current frame *)
  let get_local env n =
    let offset = Int64.mul (Int64.add n 1L) Heap.word_size in
    get_frame_ptr env ^^
      G.i (Load { ty = I64Type; align = 3; offset; sz = None})

  (* read local n of previous frame *)
  let get_prev_local env n =
    let offset = Int64.mul (Int64.add n 1L) Heap.word_size in
    (* indirect through save frame_ptr at offset 0 *)
    get_frame_ptr env ^^
    G.i (Load { ty = I64Type; align = 3; offset = 0L; sz = None}) ^^
    G.i (Load { ty = I64Type; align = 3; offset; sz = None})

  (* set local n of current frame *)
  let set_local env n =
    let offset = Int64.mul (Int64.add n 1L) Heap.word_size in
    Func.share_code1 Func.Never env ("set_local %i" ^ Int64.to_string n) ("val", I64Type) []
      (fun env get_val ->
         get_frame_ptr env ^^
         get_val ^^
         G.i (Store { ty = I64Type; align = 3; offset; sz = None}))

end (* Stack *)


module ContinuationTable = struct
  (* See rts/motoko-rts/src/closure_table.rs *)
  let remember env : G.t = E.call_import env "rts" "remember_continuation"
  let recall env : G.t = E.call_import env "rts" "recall_continuation"
  let peek_future env : G.t = E.call_import env "rts" "peek_future_continuation"
  let count env : G.t = E.call_import env "rts" "continuation_count"
  let size env : G.t = E.call_import env "rts" "continuation_table_size"
end (* ContinuationTable *)

module Bool = struct
  (* Boolean literals are either 0 or non-zero (e.g. if they origin from RTS or external API).
     They need not be shifted before put in the heap,
     because the "zero page" never contains GC-ed objects
  *)

  let vanilla_lit = function
    | false -> 0L
    | true -> 1L (* or any other non-zero value *)

  let lit b = compile_unboxed_const (vanilla_lit b)

  let lit_rts_int32 b = compile_const_32 (Int64.to_int32 (vanilla_lit b))

  let neg = compile_test I64Op.Eqz

  let from_rts_int32 =
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))

  let to_rts_int32 =
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64))

  let from_int64 =
    compile_unboxed_const 0L ^^
    compile_comparison I64Op.Ne

end (* Bool *)

module BitTagged = struct

  (* This module takes care of pointer tagging:

     A pointer to an object at offset `i` on the heap is represented as
     `i-1`, so the low two bits of the pointer are always set (0b…11).
     We call `i-1` a *skewed* pointer, in a feeble attempt to avoid the term
     shifted, which may sound like a logical shift.

     We use the constants ptr_skew and ptr_unskew to change a pointer as a
     signpost where we switch between raw pointers to skewed ones.

     This means we can store a small unboxed scalar x as (x `lsl` 1), and still
     tell it apart from a pointer by looking at the last bits: if set, it is a
     pointer.

     Small here means:

     * 0 ≤ x < 2^(ubits ty) for an unsigned type ty with (ubits ty) payload bits
     * -2^sbits ≤ x < 2^sbits, for a signed type ty with (sbits ty) (= (ubits ty) - 1) payload bits
       (i.e. excluding sign bit),
     with the exception that compact Nat is regarded as signed to support subtyping.

     Tagging needs to happen with a
     * shift left by (32-ubits ty) for a signed or unsigned type ty; then
     * a logical or of the (variable length) tag bits for ty.

     Untagging needs to happen with an
     * logical right shift (for unsigned type ty in Nat{8,16,32,64}, Char).
     * _arithmetic_ right shift (for signed type ty Int{8,16,32,64}, Int but also Nat).
       This is the right thing to do for signed numbers.
       Nat is treated as signed to allow coercion-free subtyping.

     The low bits 32 - (ubits ty) store the tag bits of the value.

     Boolean false is a non-pointer by construction.
     Boolean true (1) needs not be shifted as GC will not consider it.

     Summary:

       0b…11: A pointer
       0b…x0: A shifted scalar
       0b000: `false`
       0b001: `true`

     Note that {Nat,Int}{8,16,32} and compact {Int,Nat}64 and compact Int, Nat are explicitly tagged.
     The bits are stored in the _most_ significant bits of the `i64`,
     with the lower bits storing the variable length tag.

     {Int,Nat}64 are stored in signed and unsigned forms.

     Compact {Int,Nat} are (both) stored in signed form to support coercion free subtyping of Nat < Int.
     That means that one bit, the highest bit, of the compact Nat representation is unused and the
     representable range for both compact Int and Nat values is -2^(sbits Int) ≤ x < 2^(sbits Int).

     This describes the vanilla representation of small and compact scalars,
     used as the uniform representation of values and when stored in heap structures.

     See module TaggedSmallWord.

     The stack representation of a small scalars, UnboxedWord64 {Int,Nat}{8,16,32},
     on the other hand, always has all tag bits cleared, with the payload in the high bits of the word.

     The stack representation of compact or unboxed scalars or UnboxedWord64 {Int,Nat}64,
     on the other hand, is the natural (unpadded) machine representation.

     All arithmetic is implemented directly on the stack (not vanilla) representation of scalars.
     Proper tags bits are removed/added when loading from vanilla or storing to vanilla representation.

  *)
  let is_true_literal env =
    compile_eq_const 1L

  (* Note: `true` is not handled here, needs specific check where needed. *)
  let if_tagged_scalar env retty is1 is2 =
    compile_bitand_const 0x1L ^^
    E.if_ env retty is2 is1

  (* With two bit-tagged pointers on the stack, decide
     whether both are scalars and invoke is1 (the fast path)
     if so, and otherwise is2 (the slow path).
     Note: `true` is not handled here, needs specific check where needed.
  *)
  let if_both_tagged_scalar env retty is1 is2 =
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
    compile_bitand_const 0x1L ^^
    E.if_ env retty is2 is1

  let ubits_of pty = TaggingScheme.ubits_of pty

  let sbits_of pty = (ubits_of pty) - 1

  (* 64 bit numbers *)

  (* static *)
  let can_tag_const pty (n : int64) = Type.(
    match pty with
    | Nat | Int | Int64 | Int32 ->
      let sbits = sbits_of pty in
      let lower_bound = Int64.(neg (shift_left 1L sbits)) in
      let upper_bound = Int64.shift_left 1L sbits in
      lower_bound <= n && n < upper_bound
    | Nat64 | Nat32 ->
      let ubits = ubits_of pty in
      let upper_bound = Int64.shift_left 1L ubits in
      0L <= n && n < upper_bound
    | _ -> assert false)

  let tag_const pty i = Type.(
    match pty with
    |  Nat | Int | Int64 | Int32
    |  Nat64 | Nat32 ->
      Int64.shift_left i (64 - ubits_of pty)
      (* tag *)
      |> Int64.logor (TaggingScheme.tag_of_typ pty)
    | _ -> assert false)

  (* dynamic *)
  let sanity_check_can_tag_signed env pty get_x =
    if TaggingScheme.debug || !Flags.sanity then
      get_x ^^
      Func.share_code2 Func.Always env (prim_fun_name pty "check_can_tag_i64") (("res", I64Type), ("x", I64Type)) [I64Type]
        (fun env get_res get_x -> Type.(
          match pty with
          | Nat | Int | Int64 | Int32 ->
            let sbits = sbits_of pty in
            let lower_bound = Int64.(neg (shift_left 1L sbits)) in
            let upper_bound = Int64.shift_left 1L sbits in
            (* lower_bound <= x < upper_bound *)
            compile_unboxed_const lower_bound ^^
            get_x ^^
            compile_comparison I64Op.LeS ^^
            get_x ^^ compile_unboxed_const upper_bound ^^
            compile_comparison I64Op.LtS ^^
            G.i (Binary (Wasm_exts.Values.I64 I64Op.And))
         | Nat64 | Nat32 ->
            let ubits = ubits_of pty in
            let upper_bound = Int64.shift_left 1L ubits in
            (* 0 <= x < upper_bound *)
            get_x ^^ compile_unboxed_const upper_bound ^^
            compile_comparison I64Op.LtU
         | _ ->
            assert false) ^^
         get_res ^^
         compile_comparison I64Op.Eq ^^
         E.else_trap_with env (prim_fun_name pty "check_can_tag_i64") ^^
         get_res)
    else
      G.nop

  let if_can_tag_signed env pty retty is1 is2 = Type.(
    match pty with
    | Nat | Int | Int64 ->
      Func.share_code1 Func.Never env
        (prim_fun_name pty "if_can_tag_i64") ("x", I64Type) [I64Type] (fun env get_x ->
        (* checks that all but the low sbits are either all 0 or all 1 *)
        get_x ^^
        get_x ^^ compile_shrS_const (Int64.of_int ((64 - sbits_of pty))) ^^
        G.i (Binary (Wasm_exts.Values.I64 I32Op.Xor)) ^^
        compile_shrU_const (Int64.of_int (sbits_of pty)) ^^
        compile_test I64Op.Eqz ^^
        sanity_check_can_tag_signed env pty get_x) ^^
      E.if_ env retty is1 is2
    | Nat64 ->
      Func.share_code1 Func.Never env
         (prim_fun_name pty "if_can_tag_i64") ("x", I64Type) [I64Type] (fun env get_x ->
          (* checks that all but the low ubits are 0 *)
          get_x ^^ compile_shrU_const (Int64.of_int (ubits_of pty)) ^^
          compile_test I64Op.Eqz ^^
          sanity_check_can_tag_signed env pty get_x) ^^
      E.if_ env retty is1 is2
     | _ -> assert false)

  let if_can_tag_unsigned env pty retty is1 is2 = Type.(
    match pty with
    |  Nat | Int | Int64 ->
      let sbitsL = Int64.of_int (sbits_of pty) in
      compile_shrU_const sbitsL ^^
      compile_test I64Op.Eqz ^^
      E.if_ env retty is1 is2
    | Nat64 ->
      let ubitsL = Int64.of_int (ubits_of pty) in
      compile_shrU_const ubitsL ^^
      E.if_ env retty is2 is1 (* NB: swapped branches *)
    | _ -> assert false)

  let tag env pty = (* TBR *)
    let ubitsl = Int64.of_int (ubits_of pty) in
    compile_shl_const (Int64.sub 64L ubitsl) ^^
    (* tag *)
    compile_bitor_const (TaggingScheme.tag_of_typ pty)

  let sanity_check_tag line env ty =
    if TaggingScheme.debug || !(Flags.sanity) then
      let name =
        (prim_fun_name ty "sanity_check_tag") ^
          (if TaggingScheme.debug then Int.to_string line else "")
      in
      let tag_mask = Int64.(sub (shift_left 1L (64 - TaggingScheme.ubits_of ty)) one) in
      (Func.share_code1 Func.Always env name ("v", I64Type) [I64Type] (fun env get_n ->
         get_n ^^
         compile_bitand_const tag_mask ^^
         compile_eq_const (TaggingScheme.tag_of_typ ty) ^^
         E.else_trap_with env "unexpected tag" ^^
         get_n))
    else G.nop

  let untag line env pty = Type.(match pty with
    | Nat | Int | Int64 | Int32 | Int16 | Int8 ->
      let ubitsl = Int64.of_int (ubits_of pty) in
      sanity_check_tag line env pty ^^
      compile_shrS_const (Int64.sub 64L ubitsl)
    | Nat64 | Nat32 | Nat16 | Nat8 ->
      let ubitsl = Int64.of_int (ubits_of pty) in
      sanity_check_tag line env pty ^^
      compile_shrU_const (Int64.sub 64L ubitsl)
    | _ -> assert false)

  let clear_tag env pty =
    if TaggingScheme.tag_of_typ pty <> 0L then
      let shift_amount = 64 - ubits_of pty in
      let mask = Int64.(lognot (sub (shift_left one shift_amount) one)) in
      compile_bitand_const mask
    else G.nop

end (* BitTagged *)

module Tagged = struct
  (* Tagged objects all have an object header consisting of a tag and a forwarding pointer.
     The tag is to describe their runtime type and serves to traverse the heap
     (serialization, GC), but also for objectification of arrays.

     The tag is a word at the beginning of the object.

     The (skewed) forwarding pointer supports object moving in the incremental garbage collection.

         obj header
     ┌──────┬─────────┬──
     │ tag  │ fwd ptr │ ...
     └──────┴─────────┴──

     Attention: This mapping is duplicated in these places
       * here
       * motoko-rts/src/types.rs
       * motoko-rts/src/text.rs
       * motoko-rts/src/memory.rs
       * motoko-rts/src/bigint.rs
       * motoko-rts/src/blob-iter.rs
       * motoko-rts/src/static-checks.rs
       * In all GC implementations in motoko-rts/src/gc/
     so update all!
   *)

  type bits_sort =
    | U (* signed *)
    | S (* unsigned *)
    | F (* float *)
  type array_sort =
    | I (* [ T ] *)
    | M (* [var T ] *)
    | T (* (T,+) *)
    | S (* shared ... -> ... *)
  type blob_sort =
    | B (* Blob *)
    | T (* Text *)
    | P (* Principal *)
    | A (* actor { ... } *)

  type [@warning "-37"] tag  =
    | Object
    | Array of array_sort (* Also a tuple *)
    | Bits64 of bits_sort (* Contains a 64 bit number *)
    | MutBox (* used for mutable heap-allocated variables *)
    | Closure
    | Some (* For opt *)
    | Variant
    | Blob of blob_sort
    | Indirection (* Only used by the GC *)
    | BigInt
    | Concat (* String concatenation, used by rts/text.c *)
    | OneWordFiller (* Only used by the RTS *)
    | FreeSpace (* Only used by the RTS *)
    | Region
    | WeakRef
    | ArraySliceMinimum (* Used by the GC for incremental array marking *)
    | StableSeen (* Marker that we have seen this thing before *)
    | CoercionFailure (* Used in the Candid decoder. Static singleton! *)

  (* Tags needs to have the lowest bit set, to allow distinguishing object
     headers from heap locations (object or field addresses).

     (Reminder: objects and fields are word-aligned so will have the lowest two
     bits unset) *)
  let int_of_tag = function
    | Object -> 1L
    | Array I -> 3L
    | Array M -> 5L
    | Array T -> 7L
    | Array S -> 9L
    | Bits64 U -> 11L
    | Bits64 S -> 13L
    | Bits64 F -> 15L
    | MutBox -> 17L
    | Closure -> 19L
    | Some -> 21L
    | Variant -> 23L
    | Blob B -> 25L
    | Blob T -> 27L
    | Blob P -> 29L
    | Blob A -> 31L
    | Indirection -> 33L
    | BigInt -> 35L
    | Concat -> 37L
    | Region -> 39L
    | OneWordFiller -> 41L
    | FreeSpace -> 43L
    | WeakRef -> 45L
    | ArraySliceMinimum -> 46L
    (* Next two tags won't be seen by the GC, so no need to set the lowest bit
       for `CoercionFailure` and `StableSeen` *)
    | CoercionFailure -> 0xffff_ffff_ffff_fffeL
    | StableSeen -> 0xffff_ffff_ffff_ffffL

  (*
     The null pointer is the sentinel `0xffff_ffff_ffff_fffbL` (skewed representation).

     This serves for efficient null tests by using direct pointer comparison.
     The null pointer must not be dereferenced.
     Null tests are possible without resolving the forwarding pointer of a non-null comparand.
  *)

  let null_vanilla_pointer = 0xffff_ffff_ffff_fffbL (* skewed, pointing to last unallocated Wasm page *)
  let null_pointer = compile_unboxed_const null_vanilla_pointer

  let is_null env = compile_eq_const null_vanilla_pointer

  let not_null env =
    (* null test works without forwarding pointer resolution of a non-null comparand *)
    null_pointer ^^
    compile_comparison I64Op.Ne

  let header_size = 2L

  (* The tag *)
  let tag_field = 0L
  let forwarding_pointer_field = 1L

  (* Note: post-allocation barrier must be applied after initialization *)
  let alloc env size tag =
    assert (size > 1L);
    let name = Printf.sprintf "alloc_size<%d>_tag<%d>" (Int64.to_int size) (Int64.to_int (int_of_tag tag)) in

    Func.share_code0 Func.Never env name [I64Type] (fun env ->
      let set_object, get_object = new_local env "new_object" in
      Heap.alloc env size ^^
      set_object ^^ get_object ^^
      compile_unboxed_const (int_of_tag tag) ^^
      Heap.store_field tag_field ^^
      get_object ^^ (* object pointer *)
      get_object ^^ (* forwarding pointer *)
      Heap.store_field forwarding_pointer_field ^^
      get_object
    )

  let load_forwarding_pointer env =
    Heap.load_field forwarding_pointer_field

  let store_tag env tag =
    load_forwarding_pointer env ^^
    compile_unboxed_const (int_of_tag tag) ^^
    Heap.store_field tag_field

  let load_tag env =
    load_forwarding_pointer env ^^
    Heap.load_field tag_field

  let sanity_check_tag line env tag =
    let tag = int_of_tag tag in
    let name = "sanity_check_tag_" ^ Int64.to_string tag ^
                 (if TaggingScheme.debug then Int.to_string line else "")
    in
    if TaggingScheme.debug || !Flags.sanity then
      Func.share_code1 Func.Always env name ("obj", I64Type) [I64Type]
        (fun env get_obj ->
         get_obj ^^
         load_tag env  ^^
         compile_unboxed_const tag ^^
         compile_comparison I64Op.Eq ^^
         E.else_trap_with env name ^^
         get_obj)
    else
      G.nop

  let check_forwarding env unskewed =
    let name = "check_forwarding_" ^ if unskewed then "unskewed" else "skewed" in
    Func.share_code1 Func.Always env name ("object", I64Type) [I64Type] (fun env get_object ->
      let set_object = G.setter_for get_object in
      (if unskewed then
        get_object ^^
        compile_unboxed_const ptr_skew ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
        set_object
      else G.nop) ^^
      get_object ^^
      load_forwarding_pointer env ^^
      get_object ^^
      compile_comparison I64Op.Eq ^^
      E.else_trap_with env "missing object forwarding" ^^
      get_object ^^
      (if unskewed then
        compile_unboxed_const ptr_unskew ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
      else G.nop))

  let check_forwarding_for_store env typ =
    let (set_value, get_value, _) = new_local_ env typ "value" in
    set_value ^^ check_forwarding env false ^^ get_value

  let load_field env index =
    (if !Flags.sanity then check_forwarding env false else G.nop) ^^
    Heap.load_field index

  let store_field env index =
    (if !Flags.sanity then check_forwarding_for_store env I64Type else G.nop) ^^
    Heap.store_field index

  let load_field_float64 env index =
    (if !Flags.sanity then check_forwarding env false else G.nop) ^^
    Heap.load_field_float64 index

  let store_field_float64 env index =
    (if !Flags.sanity then check_forwarding_for_store env F64Type else G.nop) ^^
    Heap.store_field_float64 index

  (* Branches based on the tag of the object pointed to,
     leaving the object on the stack afterwards. *)
  let branch_default env retty def (cases : (tag * G.t) list) : G.t =
    let (set_tag, get_tag) = new_local env "tag" in

    let rec go = function
      | [] -> def
      | ((tag, code) :: cases) ->
        get_tag ^^
        compile_eq_const (int_of_tag tag) ^^
        E.if_ env retty code (go cases)
    in
    load_tag env ^^
    set_tag ^^
    go cases

  let allocation_barrier env =
    E.call_import env "rts" "allocation_barrier"

  let write_with_barrier env =
    let (set_value, get_value) = new_local env "written_value" in
    let (set_location, get_location) = new_local env "write_location" in
    set_value ^^ set_location ^^
    (* performance gain by first checking the GC state *)
    E.call_import env "rts" "running_gc" ^^
    Bool.from_rts_int32 ^^
    E.if0 (
      get_location ^^ get_value ^^
      E.call_import env "rts" "write_with_barrier"
    ) (
      get_location ^^ get_value ^^
      store_unskewed_ptr
    )

  let obj env tag element_instructions : G.t =
    let n = List.length element_instructions in
    let size = (Int64.add (Wasm.I64.of_int_u n) header_size) in
    let (set_object, get_object) = new_local env "new_object" in
    alloc env size tag ^^
    set_object ^^
    let init_elem idx instrs : G.t =
      get_object ^^
      instrs ^^
      Heap.store_field (Int64.add (Wasm.I64.of_int_u idx) header_size)
    in
    G.concat_mapi init_elem element_instructions ^^
    get_object ^^
    allocation_barrier env

  let shared_object line env allocation =
    let index = E.object_pool_add env line allocation in
    E.SharedObject index

  let materialize_shared_value env = function
  | E.Vanilla vanilla -> compile_unboxed_const vanilla
  | E.SharedObject index -> Heap.get_static_variable env index

  let share env allocation =
    materialize_shared_value env (shared_object __LINE__ env allocation)

end (* Tagged *)

module MutBox = struct
  (*
      Mutable heap objects

       ┌──────┬─────┬─────────┐
       │ obj header │ payload │
       └──────┴─────┴─────────┘

     The object header includes the obj tag (MutBox) and the forwarding pointer.
  *)

  let field = Tagged.header_size

  let alloc env =
    Tagged.obj env Tagged.MutBox [ compile_unboxed_zero ]

  let load_field env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env field

  let store_field env =
    let (set_mutbox_value, get_mutbox_value) = new_local env "mutbox_value" in
    set_mutbox_value ^^
    Tagged.load_forwarding_pointer env ^^
    get_mutbox_value ^^
    Tagged.store_field env field

  let add_global_mutbox env =
    E.object_pool_add env __LINE__ alloc
end



module Opt = struct
  (* The Option type. Optional values are represented as

    1. The null literal being the sentinel null pointer value, see above.

    2. ┌──────┬─────────┐
       │ some │ payload │
       └──────┴─────────┘

       A heap-allocated box for `?v` values. Should only ever contain null or
       another such box.

    3. Anything else (pointer or unboxed scalar): Constituent value, implicitly
       injected into the opt type.

    This way, `?t` is represented without allocation, with the only exception of
    the value `?ⁿnull` for n>0.

    NB: `?ⁿnull` is essentially represented by the unary encoding of the number
    of n. This could be optimized further, by storing `n` in the Some payload,
    instead of a pointer, but unlikely worth it.

  *)

  let some_payload_field = Tagged.header_size

  let null_vanilla_lit = Tagged.null_vanilla_pointer
  let null_lit env = Tagged.null_pointer

  let is_null = Tagged.is_null
  let is_some = Tagged.not_null

  let alloc_some env get_payload =
    Tagged.obj env Tagged.Some [ get_payload ]

  let inject env e =
    e ^^
    Func.share_code1 Func.Never env "opt_inject" ("x", I64Type) [I64Type] (fun env get_x ->
      get_x ^^ BitTagged.if_tagged_scalar env [I64Type]
        ( get_x ) (* scalar, no wrapping *)
        ( get_x ^^ BitTagged.is_true_literal env ^^ (* exclude true literal since `branch_default` follows the forwarding pointer *)
          E.if_ env [I64Type]
            ( get_x ) (* true literal, no wrapping *)
            ( get_x ^^ is_some env ^^
              E.if_ env [I64Type]
                ( get_x ^^ Tagged.branch_default env [I64Type]
                  ( get_x ) (* default tag, no wrapping *)
                  [ Tagged.Some, alloc_some env get_x ]
                )
                ( alloc_some env get_x ) (* ?ⁿnull for n > 0 *)
            )
        )
    )

  let constant env = function
  | E.Vanilla value when value = null_vanilla_lit -> Tagged.shared_object __LINE__ env (fun env -> alloc_some env (null_lit env)) (* ?ⁿnull for n > 0 *)
  | E.Vanilla value -> E.Vanilla value (* not null and no `Opt` object *)
  | shared_value ->
    Tagged.shared_object __LINE__ env (fun env ->
      let materialized_value = Tagged.materialize_shared_value env shared_value in
      inject env materialized_value (* potentially wrap in new `Opt` *)
    )

  (* This function is used where conceptually, Opt.inject should be used, but
  we know for sure that it wouldn’t do anything anyways, except dereferencing the forwarding pointer *)
  let inject_simple env e =
    e ^^ Tagged.load_forwarding_pointer env

  let load_some_payload_field env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env some_payload_field

  let project env =
    Func.share_code1 Func.Never env "opt_project" ("x", I64Type) [I64Type] (fun env get_x ->
      get_x ^^ BitTagged.if_tagged_scalar env [I64Type]
        ( get_x ) (* scalar, no wrapping *)
        ( get_x ^^ BitTagged.is_true_literal env ^^ (* exclude true literal since `branch_default` follows the forwarding pointer *)
          E.if_ env [I64Type]
            ( get_x ) (* true literal, no wrapping *)
            ( get_x ^^ Tagged.branch_default env [I64Type]
              ( get_x ) (* default tag, no wrapping *)
              [ Tagged.Some, get_x ^^ load_some_payload_field env ]
            )
        )
    )

end (* Opt *)


module WeakRef = struct
  (*
      Weak references

       ┌──────┬─────┬─────────┐
       │ obj header │ field │
       └──────┴─────┴─────────┘

     The object header includes the obj tag (Weak) and the forwarding pointer.
  *)

  let field = Tagged.header_size

  let alloc env =
    Tagged.obj env Tagged.WeakRef [ compile_unboxed_zero ]

  let load_field env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env field

  let store_field env =
    let (set_weak_value, get_weak_value) = new_local env "weak_value" in
    set_weak_value ^^
    Tagged.load_forwarding_pointer env ^^
    get_weak_value ^^
    Tagged.store_field env field

  let try_inject env e =
    e ^^
    Func.share_code1 Func.Never env "weak_try_inject" ("x", I64Type) [I64Type] (fun env get_x ->
      get_x ^^ Opt.is_null env ^^ E.then_trap_with env "weak reference of null" ^^
      get_x ^^ BitTagged.if_tagged_scalar env [I64Type]
       ( E.trap_with env "weak reference of non-reference"
         (* FUTURE: improve message by decoding scalar tag *)) (* scalar, trap *)
       ( get_x ^^ BitTagged.is_true_literal env ^^ (* exclude true literal since `branch_default` follows the forwarding pointer *)
          E.if_ env [I64Type]
            ( E.trap_with env "weak reference of `true`" ) (* true literal, scalar *)
            ( get_x ^^ Opt.is_some env ^^
              E.if_ env [I64Type]
                ( get_x ^^ Tagged.branch_default env [I64Type]
                  ( get_x ) (* default tag, no wrapping *)
                  [ (Tagged.Some, Opt.alloc_some env get_x);
                    (Tagged.BigInt, E.trap_with env "weak reference of Int");
                    (Tagged.Bits64 Tagged.U, E.trap_with env "weak reference of Nat64");
                    (Tagged.Bits64 Tagged.S, E.trap_with env "weak reference of Int64");
                    (Tagged.Bits64 Tagged.F, E.trap_with env "weak reference of Float") ]
                )
               ( Opt.alloc_some env get_x ) (* ?ⁿnull for n > 0 *)
            )
        )
    )


end


module Variant = struct
  (* The Variant type. We store the variant tag in a first word; we can later
     optimize and squeeze it in the Tagged tag. We can also later support unboxing
     variants with an argument of type ().

       ┌──────┬─────┬────────────┬─────────┐
       │ obj header │ varianttag │ payload │
       └──────┴─────┴────────────┴─────────┘

     The object header includes the obj tag (TAG_VARIANT) and the forwarding pointer.
  *)

  let variant_tag_field = Tagged.header_size
  let payload_field = Int64.add variant_tag_field 1L

  let hash_variant_label env : Mo_types.Type.lab -> int64 =
    E.hash env

  let inject env l e =
    Tagged.obj env Tagged.Variant [compile_unboxed_const (hash_variant_label env l); e]

  let get_variant_tag env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env variant_tag_field

  let project env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env payload_field

  (* Test if the top of the stack points to a variant with this label *)
  let test_is env l =
    get_variant_tag env ^^
    compile_eq_const (hash_variant_label env l)

end (* Variant *)


module Closure = struct
  (* In this module, we deal with closures, i.e. functions that capture parts
     of their environment.

     The structure of a closure is:

       ┌──────┬─────┬───────┬──────┬──────────────┐
       │ obj header │ funid │ size │ captured ... │
       └──────┴─────┴───────┴──────┴──────────────┘

     The object header includes the object tag (TAG_CLOSURE) and the forwarding pointer.
  *)
  let header_size = Int64.add Tagged.header_size 2L

  let funptr_field = Tagged.header_size
  let len_field = Int64.add 1L Tagged.header_size

  let load_data env i =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env (Int64.add header_size i)

  let store_data env i =
    let (set_closure_data, get_closure_data) = new_local env "closure_data" in
    set_closure_data ^^
    Tagged.load_forwarding_pointer env ^^
    get_closure_data ^^
    Tagged.store_field env (Int64.add header_size i)

  let prepare_closure_call env =
    Tagged.load_forwarding_pointer env

  (* Expect on the stack
     * the function closure (using prepare_closure_call)
     * and arguments (n-ary!)
     * the function closure again!
  *)
  let call_closure env n_args n_res =
    (* Calculate the wasm type for a given calling convention.
       An extra first argument for the closure! *)
    let ty = E.func_type env (FuncType (
      I64Type :: Lib.List.make n_args I64Type,
      FakeMultiVal.ty (Lib.List.make n_res I64Type))) in
    (* get the table index *)
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env funptr_field ^^
    (* All done: Call! *)
    let table_index = 0l in
    G.i (CallIndirect (nr table_index, nr ty)) ^^
    FakeMultiVal.load env (Lib.List.make n_res I64Type)

  let constant env get_fi =
    let fi = Wasm.I64_convert.extend_i32_u (E.add_fun_ptr env (get_fi ())) in
    Tagged.shared_object __LINE__ env (fun env -> Tagged.obj env Tagged.Closure [
      compile_unboxed_const fi;
      compile_unboxed_const 0L
    ])

end (* Closure *)


module BoxedWord64 = struct
  (* We store large word64s, nat64s and int64s in immutable boxed 64bit heap objects.

     Small values are stored unboxed, tagged, see BitTagged. The bit-tagging logic is
     contained in BitTagged; here we just do the boxing.

     The heap layout of a BoxedWord64 is:

       ┌──────┬─────┬─────┐
       │ obj header │ i64 │
       └──────┴─────┴─────┘

     The object header includes the object tag (Bits64) and the forwarding pointer.
  *)

  let payload_field = Tagged.header_size

  let heap_tag env pty =
    match pty with
    | Type.Nat64 -> Tagged.(Bits64 U)
    | Type.Int64 -> Tagged.(Bits64 S)
    | _ -> assert false

  let compile_box env pty compile_elem : G.t =
    let (set_i, get_i) = new_local env "boxed_i64" in
    let size = 4L in
    Tagged.alloc env size (heap_tag env pty) ^^
    set_i ^^
    get_i ^^ compile_elem ^^ Tagged.store_field env payload_field ^^
    get_i ^^
    Tagged.allocation_barrier env

  let constant env pty i =
    if BitTagged.can_tag_const pty i
    then
      E.Vanilla (BitTagged.tag_const pty i)
    else
      Tagged.shared_object __LINE__ env (fun env -> compile_box env pty (compile_unboxed_const i))

  let box env pty =
    Func.share_code1 Func.Never env
      (prim_fun_name pty "box64") ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^ BitTagged.if_can_tag_signed env pty [I64Type]
        (get_n ^^ BitTagged.tag env pty)
        (compile_box env pty get_n)
    )

  let unbox env pty =
    Func.share_code1 Func.Never env
      (prim_fun_name pty "unbox64") ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^
      BitTagged.if_tagged_scalar env [I64Type]
        (get_n ^^ BitTagged.untag __LINE__ env pty)
        (get_n ^^
         Tagged.load_forwarding_pointer env ^^
         Tagged.(sanity_check_tag __LINE__ env (heap_tag env pty)) ^^
         Tagged.load_field env payload_field)
    )
end (* BoxedWord64 *)

module Word64 = struct

  let compile_add env = G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
  let compile_signed_sub env = G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
  let compile_mul env = G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))
  let compile_signed_div env = G.i (Binary (Wasm_exts.Values.I64 I64Op.DivS))
  let compile_signed_mod env = G.i (Binary (Wasm_exts.Values.I64 I64Op.RemS))
  let compile_unsigned_div env = G.i (Binary (Wasm_exts.Values.I64 I64Op.DivU))
  let compile_unsigned_rem env = G.i (Binary (Wasm_exts.Values.I64 I64Op.RemU))
  let compile_unsigned_sub env =
    Func.share_code2 Func.Never env "nat_sub" (("n1", I64Type), ("n2", I64Type)) [I64Type] (fun env get_n1 get_n2 ->
      get_n1 ^^ get_n2 ^^ compile_comparison I64Op.LtU ^^
      E.then_trap_with env "Natural subtraction underflow" ^^
      get_n1 ^^ get_n2 ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
    )

  let compile_unsigned_pow env =
    let name = prim_fun_name Type.Nat64 "wpow_nat" in
    Func.share_code2 Func.Always env name (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        let set_n = G.setter_for get_n in
        let set_exp = G.setter_for get_exp in
        let (set_acc, get_acc) = new_local env "acc" in

        (* start with result = 1 *)
        compile_unboxed_one ^^ set_acc ^^

        (* handle exp == 0 *)
        get_exp ^^ compile_test I64Op.Eqz ^^
        E.if1 I64Type get_acc (* done *)
        begin
          G.loop0 begin
            (* Are we done? *)
            get_exp ^^ compile_unboxed_one ^^ compile_comparison I64Op.LeU ^^
            E.if0 G.nop (* done *)
            begin
              (* Check low bit of exp to see if we need to multiply *)
              get_exp ^^ compile_shl_const 63L ^^ compile_test I64Op.Eqz ^^
              E.if0 G.nop
              begin
                (* Multiply! *)
                get_acc ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^ set_acc
              end ^^
              (* Square n, and shift exponent *)
              get_n ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^ set_n ^^
              get_exp ^^ compile_shrU_const 1L ^^ set_exp ^^
              (* And loop *)
              G.i (Br (nr 1l))
            end
          end ^^
          (* Multiply a last time *)
          get_acc ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))
        end
      )


  let compile_signed_wpow env =
    Func.share_code2 Func.Never env "wrap_pow_Int64" (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        get_exp ^^
        compile_unboxed_const 0L ^^
        compile_comparison I64Op.GeS ^^
        E.else_trap_with env "negative power" ^^
        get_n ^^ get_exp ^^ compile_unsigned_pow env
      )

  let _compile_eq env = compile_comparison I64Op.Eq
  let compile_relop env i64op = compile_comparison i64op

  let btst_kernel env =
    let (set_b, get_b) = new_local env "b" in
    set_b ^^ compile_unboxed_one ^^ get_b ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)) ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.And))

end (* BoxedWord64 *)

module TaggedSmallWord = struct
  (* While smaller-than-64bit words are treated as i64 from the WebAssembly
     perspective, there are certain differences that are type based. This module
     provides helpers to abstract over those.

     Caution: Some functions here are also used for unboxed Nat64/Int64, while others
     are _only_ used for the small ones. Check call-sites!
  *)

  let toNat = Type.(function
    | Int8 | Nat8 -> Nat8
    | Int16 | Nat16 -> Nat16
    | Int32 | Nat32 -> Nat32
    | Int64 | Nat64 -> Nat64
    | _ -> assert false)

  let bits_of_type = Type.(function
    | Int8 | Nat8 -> 8
    | Int16 | Nat16 -> 16
    | Char -> 21
    | Nat32 | Int32 -> 32
    (* unboxed on stack *)
    | Nat64 | Int64 -> 64
    | _  -> assert false)

  let tag_of_type pty = Type.(match pty with
    | Int8 | Nat8
    | Int16 | Nat16
    | Int32 | Nat32
    | Char ->
      TaggingScheme.tag_of_typ pty
    (* unboxed on stack *)
    | Int64 | Nat64 -> 0L
    | _ -> assert false)

  let shift_of_type ty = Int64.of_int (64 - bits_of_type ty)

  let bitwidth_mask_of_type = function
    | Type.(Int8|Nat8) -> 0b111L
    | Type.(Int16|Nat16) -> 0b1111L
    | Type.(Int32|Nat32) -> 0b11111L
    | p -> todo "bitwidth_mask_of_type" (Arrange_type.prim p) 0L

  let const_of_type ty n = Int64.(shift_left n (to_int (shift_of_type ty)))

  let padding_of_type ty = Int64.(sub (const_of_type ty 1L) one)

  let mask_of_type ty = Int64.lognot (padding_of_type ty)

  (* Makes sure that we only shift/rotate the maximum number of bits available in the word. *)
  let clamp_shift_amount = function
    | Type.(Nat64|Int64) -> G.nop
    | ty -> compile_bitand_const (bitwidth_mask_of_type ty)

  let shift_leftWordNtoI64 = compile_shl_const

  (* Makes sure that the word payload (e.g. shift/rotate amount) is in the LSB bits of the word. *)
  let lsb_adjust = function
    | Type.(Int64|Nat64) -> G.nop
    | Type.(Nat8|Nat16|Nat32) as ty -> compile_shrU_const (shift_of_type ty)
    | Type.(Int8|Int16|Int32) as ty -> compile_shrS_const (shift_of_type ty)
    | Type.Char as ty -> compile_shrU_const (shift_of_type ty)
    | _ -> assert false

  (* Makes sure that the word payload (e.g. operation result) is in the MSB bits of the word. *)
  let msb_adjust = function
    | Type.(Int64|Nat64) -> G.nop
    | ty -> shift_leftWordNtoI64 (shift_of_type ty)

  (* Makes sure that the word representation invariant is restored. *)
  let sanitize_word_result = function
    | Type.(Nat64|Int64) -> G.nop
    | ty -> compile_bitand_const (mask_of_type ty)

  (* Sets the number (according to the type's word invariant) of LSBs. *)
  let compile_word_padding = function
    | Type.(Nat64|Int64) -> G.nop
    | ty -> compile_bitor_const (padding_of_type ty)

  (* Kernel for counting leading zeros, according to the word invariant. *)
  let clz_kernel ty =
    compile_word_padding ty ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
    msb_adjust ty

  (* Kernel for counting trailing zeros, according to the word invariant. *)
  let ctz_kernel ty =
    compile_word_padding ty ^^
    compile_rotr_const (shift_of_type ty) ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Ctz)) ^^
    msb_adjust ty

  (* Kernel for testing a bit position, according to the word invariant. *)
  let btst_kernel env ty =
    let (set_b, get_b) = new_local env "b"
    in lsb_adjust ty ^^ set_b ^^ lsb_adjust ty ^^
       compile_unboxed_one ^^ get_b ^^ clamp_shift_amount ty ^^
       G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)) ^^
       G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
       msb_adjust ty

  (* Code points occupy 21 bits, so can always be tagged scalars *)
  let lsb_adjust_codepoint env = lsb_adjust Type.Char
  let msb_adjust_codepoint = msb_adjust Type.Char

  (* Checks (n < 0xD800 || 0xE000 ≤ n ≤ 0x10FFFF),
     ensuring the codepoint range and the absence of surrogates. *)
  let check_and_msb_adjust_codepoint env =
    Func.share_code1 Func.Always env "Nat32->Char" ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^ compile_unboxed_const 0xD800L ^^
      compile_comparison I64Op.GeU ^^
      get_n ^^ compile_unboxed_const 0xE000L ^^
      compile_comparison I64Op.LtU ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
      get_n ^^ compile_unboxed_const 0x10FFFFL ^^
      compile_comparison I64Op.GtU ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
      E.then_trap_with env "codepoint out of range" ^^
      get_n ^^ msb_adjust_codepoint
    )

  let vanilla_lit ty v =
    Int64.(shift_left v (to_int (shift_of_type ty)))
    |> Int64.logor (tag_of_type ty)

  (* Wrapping implementation for multiplication and exponentiation. *)

  let compile_word_mul env ty =
    lsb_adjust ty ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))

  let compile_nat_power env ty =
    let name = prim_fun_name ty "wpow_nat" in
    (* Square- and multiply exponentiation *)
    Func.share_code2 Func.Always env name (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        let set_n = G.setter_for get_n in
        let set_exp = G.setter_for get_exp in
        let (set_acc, get_acc) = new_local env "acc" in

        (* unshift arguments *)
        get_exp ^^ compile_shrU_const (shift_of_type ty) ^^ set_exp ^^
        get_n ^^ compile_shrU_const (shift_of_type ty) ^^ set_n ^^

        (* The accumulator starts with and stays shifted, so no other shifts needed. *)
        compile_unboxed_const (const_of_type ty 1L) ^^ set_acc ^^

        (* handle exp == 0 *)
        get_exp ^^ compile_test I64Op.Eqz ^^
        E.if1 I64Type get_acc (* done *)
        begin
          G.loop0 begin
            (* Are we done? *)
            get_exp ^^ compile_unboxed_one ^^ compile_comparison I64Op.LeU ^^
            E.if0 G.nop (* done *)
            begin
              (* Check low bit of exp to see if we need to multiply *)
              get_exp ^^ compile_shl_const 63L ^^ compile_test I64Op.Eqz ^^
              E.if0 G.nop
              begin
                (* Multiply! *)
                get_acc ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^ set_acc
              end ^^
              (* Square n, and shift exponent *)
              get_n ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^ set_n ^^
              get_exp ^^ compile_shrU_const 1L ^^ set_exp ^^
              (* And loop *)
              G.i (Br (nr 1l))
            end
          end ^^
          (* Multiply a last time *)
          get_acc ^^ get_n ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))
          (* Accumulator was shifted, so no further shift needed here *)
        end
      )

  let compile_int_power env ty =
    let name = prim_fun_name ty "wpow_int" in
    Func.share_code2 Func.Never env name (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        get_exp ^^
        compile_unboxed_const 0L ^^
        compile_comparison I64Op.GeS ^^
        E.else_trap_with env "negative power" ^^
        get_n ^^ get_exp ^^ compile_nat_power env (toNat ty))

  (* To rotate, first rotate a copy by bits_of_type into the other direction *)
  let rotl env ty =
     Func.share_code2 Func.Never env (prim_fun_name ty "rotl") (("n", I64Type), ("by", I64Type)) [I64Type]
       (fun env get_n get_by ->
        let open Wasm_exts.Values in
        let beside_adjust = compile_rotr_const (Int64.of_int (bits_of_type ty)) in
        get_n ^^ get_n ^^ beside_adjust ^^ G.i (Binary (I64 I64Op.Or)) ^^
        get_by ^^ lsb_adjust ty ^^ clamp_shift_amount ty ^^ G.i (Binary (I64 I64Op.Rotl)) ^^
        sanitize_word_result ty
       )

  let rotr env ty =
     Func.share_code2 Func.Never env (prim_fun_name ty "rotr") (("n", I64Type), ("by", I64Type)) [I64Type]
       (fun env get_n get_by ->
        let open Wasm_exts.Values in
        let beside_adjust = compile_rotl_const (Int64.of_int (bits_of_type ty)) in
        get_n ^^ get_n ^^ beside_adjust ^^ G.i (Binary (I64 I64Op.Or)) ^^
        get_by ^^ lsb_adjust ty ^^ clamp_shift_amount ty ^^ G.i (Binary (I64 I64Op.Rotr)) ^^
        sanitize_word_result ty
       )

  let tag env pty =
    match pty with
    | Type.(Nat8 | Int8 | Nat16 | Int16 | Nat32 | Int32 | Char) ->
      (* TODO:  could sanity check low bits clear *)
      (* add tag *)
      compile_bitor_const (tag_of_type pty)
    | _ -> assert false

  let untag env pty =
    match pty with
    | Type.(Nat8 | Int8 | Nat16 | Int16 | Nat32 | Int32 | Char) ->
       (* check tag *)
       BitTagged.sanity_check_tag __LINE__ env pty ^^
       (* clear tag *)
       BitTagged.clear_tag env pty
    | _ -> assert false

end (* TaggedSmallWord *)


module Float = struct
  (* We store floats (C doubles) in immutable boxed 64bit heap objects.

     The heap layout of a Float is:

       ┌──────┬─────┬─────┐
       │ obj header │ f64 │
       └──────┴─────┴─────┘

     For now the tag stored is that of a Bits64, because the payload is
     treated opaquely by the RTS. We'll introduce a separate tag when the need of
     debug inspection (or GC representation change) arises.

     The object header includes the object tag (Bits64) and the forwarding pointer.
  *)

  let payload_field = Tagged.header_size

  let compile_unboxed_const f = G.i (Const (nr (Wasm_exts.Values.F64 f)))

  let box env = Func.share_code1 Func.Never env "box_f64" ("f", F64Type) [I64Type] (fun env get_f ->
    let (set_i, get_i) = new_local env "boxed_f64" in
    let size = Int64.add Tagged.header_size 2L in
    Tagged.alloc env size Tagged.(Bits64 F) ^^
    set_i ^^
    get_i ^^ get_f ^^ Tagged.store_field_float64 env payload_field ^^
    get_i ^^
    Tagged.allocation_barrier env
  )

  let unbox env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.(sanity_check_tag __LINE__ env (Bits64 F)) ^^
    Tagged.load_field_float64 env payload_field

  let constant env f = Tagged.shared_object __LINE__ env (fun env ->
    compile_unboxed_const f ^^
    box env)

end (* Float *)


module ReadBuf = struct
  (*
  Combinators to safely read from a dynamic buffer.

  We represent a buffer by a pointer to two words in memory (usually allocated
  on the shadow stack): The first is a pointer to the current position of the buffer,
  the second one a pointer to the end (to check out-of-bounds).

  Code that reads from this buffer will update the former, i.e. it is mutable.

  The format is compatible with C (pointer to a struct) and avoids the need for the
  multi-value extension that we used before to return both parse result _and_
  updated pointer.

  All pointers here are unskewed!

  This module is mostly for serialization, but because there are bits of
  serialization code in the BigNumType implementations, we put it here.
  *)

  let get_ptr get_buf =
    get_buf ^^ G.i (Load {ty = I64Type; align = 3; offset = 0L; sz = None})
  let get_end get_buf =
    get_buf ^^ G.i (Load {ty = I64Type; align = 3; offset = Heap.word_size; sz = None})
  let set_ptr get_buf new_val =
    get_buf ^^ new_val ^^ G.i (Store {ty = I64Type; align = 3; offset = 0L; sz = None})
  let set_end get_buf new_val =
    get_buf ^^ new_val ^^ G.i (Store {ty = I64Type; align = 3; offset = Heap.word_size; sz = None})
  let set_size get_buf get_size =
    set_end get_buf
      (get_ptr get_buf ^^ get_size ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)))

  let alloc env f = Stack.with_words env "buf" 2L f

  let advance get_buf get_delta =
    set_ptr get_buf (get_ptr get_buf ^^ get_delta ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)))

  let read_leb128 env get_buf =
    get_buf ^^ E.call_import env "rts" "leb128_decode"

  let read_sleb128 env get_buf =
    get_buf ^^ E.call_import env "rts" "sleb128_decode"

  let check_space env get_buf get_delta =
    get_delta ^^
    get_end get_buf ^^ get_ptr get_buf ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
    compile_comparison I64Op.LeU ^^
    E.else_trap_with env "IDL error: out of bounds read"

  let check_page_end env get_buf incr_delta =
    get_ptr get_buf ^^ compile_bitand_const 0xFFFFL ^^
    incr_delta ^^
    compile_shrU_const 16L

  let is_empty env get_buf =
    get_end get_buf ^^ get_ptr get_buf ^^
    compile_comparison I64Op.Eq

  let read_byte env get_buf =
    check_space env get_buf compile_unboxed_one ^^
    get_ptr get_buf ^^
    G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack8, ZX)}) ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    advance get_buf compile_unboxed_one

  let read_word16 env get_buf =
    check_space env get_buf (compile_unboxed_const 2L) ^^
    get_ptr get_buf ^^
    G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack16, ZX)}) ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    advance get_buf (compile_unboxed_const 2L)

  let read_word32 env get_buf =
    check_space env get_buf (compile_unboxed_const 4L) ^^
    get_ptr get_buf ^^
    G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = None}) ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    advance get_buf (compile_unboxed_const 4L)

  let read_signed_word32 env get_buf =
    check_space env get_buf (compile_unboxed_const 4L) ^^
    get_ptr get_buf ^^
    G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = None}) ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendSI32)) ^^
    advance get_buf (compile_unboxed_const 4L)

  let speculative_read_word64 env get_buf =
    check_page_end env get_buf (compile_add_const 8L) ^^
    E.if1 I64Type
      (compile_unboxed_const (-1L))
      begin
        get_ptr get_buf ^^
        G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = None})
      end

  let read_word64 env get_buf =
    check_space env get_buf (compile_unboxed_const 8L) ^^
    get_ptr get_buf ^^
    G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = None}) ^^
    advance get_buf (compile_unboxed_const 8L)

  let read_float64 env get_buf =
    check_space env get_buf (compile_unboxed_const 8L) ^^
    get_ptr get_buf ^^
    G.i (Load {ty = F64Type; align = 0; offset = 0L; sz = None}) ^^
    advance get_buf (compile_unboxed_const 8L)

  let read_blob env get_buf get_len =
    check_space env get_buf get_len ^^
    (* Already has destination address on the stack *)
    get_ptr get_buf ^^
    get_len ^^
    Heap.memcpy env ^^
    advance get_buf get_len

end (* Buf *)


type comparator = Lt | Le | Ge | Gt

module type BigNumType =
sig
  (* word from SR.Vanilla, trapping, unsigned semantics *)
  val to_word64 : E.t -> G.t
  val to_word64_with : E.t -> G.t -> G.t (* with error message on stack (ptr/len) *)

  (* word from SR.Vanilla, lossy, raw bits *)
  val truncate_to_word32 : E.t -> G.t
  val truncate_to_word64 : E.t -> G.t

  (* unsigned word to SR.Vanilla *)
  val from_word64 : E.t -> G.t

  (* signed word to SR.Vanilla *)
  val from_signed_word_compact : E.t -> G.t
  val from_signed_word64 : E.t -> G.t

  (* buffers *)
  (* given a numeric object on stack (vanilla),
     push the number (i64) of bytes necessary
     to externalize the numeric object *)
  val compile_data_size_signed : E.t -> G.t
  val compile_data_size_unsigned : E.t -> G.t
  (* given on stack
     - numeric object (vanilla, TOS)
     - data buffer
    store the binary representation of the numeric object into the data buffer,
    and push the number (i64) of bytes stored onto the stack
   *)
  val compile_store_to_data_buf_signed : E.t -> G.t
  val compile_store_to_data_buf_unsigned : E.t -> G.t
  (* given a ReadBuf on stack, consume bytes from it,
     deserializing to a numeric object
     and leave it on the stack (vanilla).
     The boolean argument is true if the value to be read is signed.
   *)
  val compile_load_from_data_buf : E.t -> G.t -> bool -> G.t

  (* constant *)
  val constant : E.t -> Big_int.big_int -> E.shared_value

  (* arithmetic *)
  val compile_abs : E.t -> G.t
  val compile_neg : E.t -> G.t
  val compile_add : E.t -> G.t
  val compile_signed_sub : E.t -> G.t
  val compile_unsigned_sub : E.t -> G.t
  val compile_mul : E.t -> G.t
  val compile_signed_div : E.t -> G.t
  val compile_signed_mod : E.t -> G.t
  val compile_unsigned_div : E.t -> G.t
  val compile_unsigned_rem : E.t -> G.t
  val compile_unsigned_pow : E.t -> G.t
  val compile_lsh : E.t -> G.t
  val compile_rsh : E.t -> G.t

  (* comparisons *)
  val compile_eq : E.t -> G.t
  val compile_is_negative : E.t -> G.t
  val compile_relop : E.t -> comparator -> G.t

  (* representation checks *)
  (* given a numeric object on the stack as skewed pointer, check whether
     it can be faithfully stored in N bits, including a leading sign bit
     leaves boolean result on the stack
     N must be 2..64
   *)
  val fits_signed_bits : E.t -> int -> G.t
  (* given a numeric object on the stack as skewed pointer, check whether
     it can be faithfully stored in N unsigned bits
     leaves boolean result on the stack
     N must be 1..64
   *)
  val fits_unsigned_bits : E.t -> int -> G.t
end

let i64op_from_relop = function
  | Lt -> I64Op.LtS
  | Le -> I64Op.LeS
  | Ge -> I64Op.GeS
  | Gt -> I64Op.GtS

let name_from_relop = function
  | Lt -> "B_lt"
  | Le -> "B_le"
  | Ge -> "B_ge"
  | Gt -> "B_gt"

(* helper, measures the dynamics of the unsigned i64, returns (64 - effective bits) *)
let unsigned_dynamics get_x =
  get_x ^^
  G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz))

(* helper, measures the dynamics of the signed i64, returns (64 - effective bits) *)
let signed_dynamics get_x =
  get_x ^^ compile_shl_const 1L ^^
  get_x ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^
  G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz))

module I32Leb = struct
  let compile_size dynamics get_x =
    get_x ^^ Bool.from_int64 ^^
    E.if1 I64Type
      begin
        (* Add (7-1) to prepare division by 7 that is rounded up *)
        compile_unboxed_const (Int64.of_int (Int.add 64 (Int.sub 7 1))) ^^
        dynamics get_x ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
        compile_divU_const 7L
      end
      compile_unboxed_one

  let compile_leb128_size get_x = compile_size unsigned_dynamics get_x
  let compile_sleb128_size get_x = compile_size signed_dynamics get_x

  let compile_store_to_data_buf_unsigned env get_x get_buf =
    get_x ^^ get_buf ^^
    E.call_import env "rts" "leb128_encode" ^^
    compile_leb128_size get_x

  let compile_store_to_data_buf_signed env get_x get_buf =
    get_x ^^ get_buf ^^
    E.call_import env "rts" "sleb128_encode" ^^
    compile_sleb128_size get_x
end

module MakeCompact (Num : BigNumType) : BigNumType = struct

  (* Compact BigNums are a representation of signed BitTagged.ubit-bignums (of the
     underlying boxed representation `Num`), that fit into an i64 as per the
     BitTagged representation.

     Many arithmetic operations can be be performed on this right-zero-padded
     representation directly. For some operations (e.g. multiplication) the
     second argument needs to be furthermore right-shifted to avoid overflow.
     Similarly, for division the result must be left-shifted.

     Generally all operations begin with checking whether both arguments are
     already tagged scalars. If so, the arithmetic can be performed in machine
     registers (fast path). Otherwise one or both arguments need boxing and the
     arithmetic needs to be carried out on the underlying boxed bignum
     representation (slow path).

     The result appears as a boxed number in the latter case, so a check is
     performed if it can be a tagged scalar. Conversely in the former case the
     64-bit result can either be a tagged scalar or needs to be boxed.

     Manipulation of the result is unnecessary for the comparison predicates.

     For the `pow` operation the check that both arguments are tagged scalars
     is not sufficient. Here we count and multiply effective bitwidths to
     figure out whether the operation will overflow 64 bits, and if so, we fall
     back to the slow path.
   *)

  (* TODO: There is some unnecessary result shifting when the div result needs
     to be boxed. Is this possible at all to happen? With (/-1) maybe! *)

  (* TODO: Does the result of the rem/mod fast path ever needs boxing? *)

  (* examine the skewed pointer and determine if number fits into ubits *)
  let fits_in_vanilla env = Num.fits_signed_bits env (BitTagged.ubits_of Type.Int)

  let clear_tag env = BitTagged.clear_tag env Type.Int

  (* A variant of BitTagged.can_tag that works on signed i64 *)
  let if_can_tag env retty is1 is2 =
    let ubitsL = Int64.of_int(BitTagged.ubits_of Type.Int) in
    compile_shrS_const (Int64.sub 64L ubitsL) ^^ BitTagged.if_can_tag_signed env Type.Int retty is1 is2

  let apply_tag env =
    compile_bitor_const (TaggingScheme.tag_of_typ Type.Int)

  let can_use_fath_path env get_a get_b =
    (* Check whether both arguments `a` and `b` are scalars that fit within 32 bit.
        This is to guarantee overflow-free 64-bit arithmetics, such as `add`, `sub`, or `mul`.
        However, this does not work for `pow` as it can overflow for smaller arguments. *)
    (* check with a combined bit mask that:
       - (and `0x1`) Both arguments are scalars, none a skewed pointers
       - (and `0xFFFF_FFFF_0000_0000`) Both arguments fit in 32-bit
    TODO: Precise tag for Int has 2 bits ->
       Check if we could permit one or two more bits in the `0xFFFF_FFFF_0000_0000` bit mask. *)
    get_a ^^ get_b ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
    compile_bitand_const 0xFFFF_FFFF_0000_0001L ^^
    compile_eq_const 0x0L

  (* creates a boxed bignum from a signed i64 *)
  let box env =
    let ubitsL = Int64.of_int(BitTagged.ubits_of Type.Int) in
    compile_shrS_const (Int64.sub 64L ubitsL) ^^ Num.from_signed_word64 env

  (* check if both arguments are tagged scalars,
     if so, perform the fast path.
     Otherwise make sure that both arguments are in heap representation,
     and run the slow path on them.
     In both cases bring the results into normal form.
   *)
  let try_unbox2 name fast slow env =
    Func.share_code2 Func.Always env name (("a", I64Type), ("b", I64Type)) [I64Type]
      (fun env get_a get_b ->
        let set_res, get_res = new_local env "res" in
        can_use_fath_path env get_a get_b ^^
        E.if1 I64Type
          begin
            get_a ^^ clear_tag env ^^
            get_b ^^ clear_tag env ^^
            fast env ^^ set_res ^^
            get_res ^^
            if_can_tag env [I64Type]
              (get_res ^^ apply_tag env)
              (get_res ^^ box env)
          end
          begin
            get_a ^^ BitTagged.if_tagged_scalar env [I64Type]
              (get_a ^^ box env)
              get_a ^^
            get_b ^^ BitTagged.if_tagged_scalar env [I64Type]
              (get_b ^^ box env)
              get_b ^^
            slow env ^^ set_res ^^ get_res ^^
            fits_in_vanilla env ^^
            E.if1 I64Type
              (get_res ^^ Num.truncate_to_word64 env ^^ BitTagged.tag env Type.Int)
              get_res
          end
      )

  let compile_add = try_unbox2 "B_add" Word64.compile_add Num.compile_add

  let adjust_arg2 code env =
    compile_shrS_const (Int64.of_int (64 - BitTagged.ubits_of Type.Int)) ^^
    code env (* TBR *)
  let adjust_result code env =
    code env ^^
    compile_shl_const (Int64.of_int (64 - BitTagged.ubits_of Type.Int))

  let compile_mul = try_unbox2 "B_mul" (adjust_arg2 Word64.compile_mul) Num.compile_mul
  let compile_signed_sub = try_unbox2 "B+sub" Word64.compile_signed_sub Num.compile_signed_sub
  let compile_signed_div = try_unbox2 "B+div" (adjust_result Word64.compile_signed_div) Num.compile_signed_div
  let compile_signed_mod = try_unbox2 "B_mod" Word64.compile_signed_mod Num.compile_signed_mod
  let compile_unsigned_div = try_unbox2 "B_div" (adjust_result Word64.compile_unsigned_div) Num.compile_unsigned_div
  let compile_unsigned_rem = try_unbox2 "B_rem" Word64.compile_unsigned_rem Num.compile_unsigned_rem
  let compile_unsigned_sub = try_unbox2 "B_sub" Word64.compile_unsigned_sub Num.compile_unsigned_sub

  let compile_unsigned_pow env =
    Func.share_code2 Func.Always env "B_pow" (("a", I64Type), ("b", I64Type)) [I64Type]
    (fun env get_a get_b ->
    let set_res, get_res = new_local env "res" in
    get_a ^^ get_b ^^
    BitTagged.if_both_tagged_scalar env [I64Type]
      begin
        let set_a = G.setter_for get_a in
        let set_b = G.setter_for get_b in
        (* Convert to plain Word64 *)
        get_a ^^ BitTagged.untag __LINE__ env Type.Int ^^ set_a ^^
        get_b ^^ BitTagged.untag __LINE__ env Type.Int ^^ set_b ^^

        get_a ^^ Num.from_signed_word64 env ^^
        get_b ^^ Num.from_signed_word64 env ^^
        Num.compile_unsigned_pow env ^^ set_res ^^
        get_res ^^ fits_in_vanilla env ^^
        E.if1 I64Type
          (get_res ^^ Num.truncate_to_word64 env ^^ BitTagged.tag env Type.Int)
          get_res
      end
      begin
        get_a ^^ BitTagged.if_tagged_scalar env [I64Type]
          (get_a ^^ box env)
          get_a ^^
        get_b ^^ BitTagged.if_tagged_scalar env [I64Type]
          (get_b ^^ box env)
          get_b ^^
        Num.compile_unsigned_pow env ^^ set_res ^^
        get_res ^^ fits_in_vanilla env ^^
        E.if1 I64Type
          (get_res ^^ Num.truncate_to_word64 env ^^ BitTagged.tag env Type.Int)
          get_res
      end)

  (*
    Note [left shifting compact Nat]
    For compact Nats with a number fitting in 32 bits (in scalar value representation) and a shift amount of
    less or equal 32, we perform a fast shift. Otherwise, the bignum shift via RTS is applied.
   *)
  let compile_lsh env =
    Func.share_code2 Func.Always env "B_lsh" (("n", I64Type), ("amount", I64Type)) [I64Type]
    (fun env get_n get_amount ->
      let set_n = G.setter_for get_n in
      get_amount ^^ TaggedSmallWord.lsb_adjust Type.Nat32 ^^ G.setter_for get_amount ^^
      get_n ^^
      BitTagged.if_tagged_scalar env [I64Type]
      begin
        (* see Note [left shifting compact Nat] *)
        get_n ^^ BitTagged.untag __LINE__ env Type.Int ^^ set_n ^^
        get_n ^^
        compile_bitand_const 0xFFFF_FFFF_0000_0000L ^^
        compile_eq_const 0L ^^
        get_amount ^^ compile_rel_const I64Op.LeU 32L ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
        E.if1 I64Type
        begin
          get_n ^^
          get_amount ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)) ^^
          BitTagged.tag env Type.Int
        end
        begin
          get_n ^^ Num.from_word64 env ^^
          get_amount ^^
          Num.compile_lsh env
        end
      end
      begin
        get_n ^^ get_amount ^^ Num.compile_lsh env
      end)

  let compile_rsh env =
    Func.share_code2 Func.Always env "B_rsh" (("n", I64Type), ("amount", I64Type)) [I64Type]
    (fun env get_n get_amount ->
      get_amount ^^ TaggedSmallWord.lsb_adjust Type.Nat32 ^^ G.setter_for get_amount ^^
      get_n ^^
      BitTagged.if_tagged_scalar env [I64Type]
        begin
          get_n ^^ clear_tag env ^^
          get_amount ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrU)) ^^
          compile_bitand_const Int64.(shift_left minus_one (64 - BitTagged.ubits_of Type.Int)) ^^
          get_amount ^^ compile_rel_const I64Op.LeU (Int64.of_int (BitTagged.ubits_of Type.Int))^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) (* branch-free `if` *) ^^
          (* tag *)
          apply_tag env
        end
        begin
          get_n ^^ get_amount ^^ Num.compile_rsh env ^^
          let set_res, get_res = new_local env "res" in
          set_res ^^ get_res ^^
          fits_in_vanilla env ^^
          E.if1 I64Type
            (get_res ^^ Num.truncate_to_word64 env ^^ BitTagged.tag env Type.Int)
            get_res
        end)

  let compile_is_negative env =
    let set_n, get_n = new_local env "n" in
    set_n ^^ get_n ^^
    BitTagged.if_tagged_scalar env [I64Type]
      (get_n ^^ clear_tag env ^^ compile_unboxed_const 0L ^^ compile_comparison I64Op.LtS)
      (get_n ^^ Num.compile_is_negative env)

  let constant env = function
    | n when Big_int.is_int_big_int n && BitTagged.can_tag_const Type.Int (Big_int.int64_of_big_int n) ->
      E.Vanilla (BitTagged.tag_const Type.Int (Big_int.int64_of_big_int n))
    | n -> Num.constant env n

  let compile_neg env =
    let sminl = Int64.shift_left 1L (BitTagged.sbits_of Type.Int) in
    let sminl_shifted = Int64.shift_left sminl (64 - BitTagged.ubits_of Type.Int) in
    Func.share_code1 Func.Always env "B_neg" ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^ BitTagged.if_tagged_scalar env [I64Type]
        begin
          get_n ^^ clear_tag env ^^ compile_eq_const sminl_shifted ^^ (* -2^sbits, shifted ubits *)
          E.if1 I64Type
            (compile_unboxed_const sminl ^^ Num.from_word64 env)
            begin
              compile_unboxed_const 0L ^^
              get_n ^^ clear_tag env ^^
              G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
              (* tag the result *)
              clear_tag env ^^
              apply_tag env
            end
        end
        (get_n ^^ Num.compile_neg env)
    )

  let try_comp_unbox2 name fast slow env =
    Func.share_code2 Func.Always env name (("a", I64Type), ("b", I64Type)) [I64Type]
      (fun env get_a get_b ->
        get_a ^^ get_b ^^
        BitTagged.if_both_tagged_scalar env [I64Type]
          begin
            get_a ^^ clear_tag env ^^
            get_b ^^ clear_tag env ^^
            fast env
          end
          begin
            get_a ^^ BitTagged.if_tagged_scalar env [I64Type]
              (get_a ^^ box env)
              get_a ^^
            get_b ^^ BitTagged.if_tagged_scalar env [I64Type]
              (get_b ^^ box env)
              get_b ^^
            slow env
          end)

  let compile_eq env =
    Func.share_code2 Func.Always env "B_eq" (("a", I64Type), ("b", I64Type)) [I64Type]
      (fun env get_a get_b ->
        get_a ^^ get_b ^^
        compile_comparison I64Op.Eq ^^
        E.if1 I64Type
          (Bool.lit true)
          (get_a ^^ get_b ^^
           BitTagged.if_both_tagged_scalar env [I64Type]
             (Bool.lit false)
             begin
               get_a ^^ BitTagged.if_tagged_scalar env [I64Type]
                 (get_a ^^ box env)
                 get_a ^^
               get_b ^^ BitTagged.if_tagged_scalar env [I64Type]
                 (get_b ^^ box env)
                 get_b ^^
               Num.compile_eq env
             end))

  let compile_relop env bigintop =
    try_comp_unbox2 (name_from_relop bigintop)
      (fun env' -> Word64.compile_relop env' (i64op_from_relop bigintop))
      (fun env' -> Num.compile_relop env' bigintop)
      env

  let try_unbox iN fast slow env =
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    BitTagged.if_tagged_scalar env [iN]
      (get_a ^^ fast env)
      (get_a ^^ slow env)

  let fits_unsigned_bits env n =
    try_unbox I64Type (fun _ -> match n with
        | 64 -> G.i Drop ^^ Bool.lit true
        | 8 | 16 | 32 ->
          (* use shifting to test that the payload including the tag fits the desired bit width.
              E.g. this is now n + 2 for Type.Int. *)
          compile_bitand_const Int64.(shift_left minus_one (n + (64 - BitTagged.ubits_of Type.Int))) ^^
          compile_test I64Op.Eqz
        | _ -> assert false
      )
      (fun env -> Num.fits_unsigned_bits env n)
      env

  let sanity_check_fits_signed_bits env n get_a =
     if TaggingScheme.debug || !Flags.sanity then
     get_a ^^
     Func.share_code2 Func.Always env ("check_fits_signed_bits_"^Int.to_string n) (("res", I64Type), ("a", I64Type)) [I64Type]
      (fun env get_res get_a ->
         let lower_bound = Int64.(neg (shift_left 1L (n-1))) in
         let upper_bound = Int64.shift_left 1L (n-1) in
         let set_a = G.setter_for get_a in
         get_a ^^
         compile_shrS_const (Int64.of_int (64 - BitTagged.ubits_of Type.Int)) ^^
         set_a ^^
         compile_unboxed_const lower_bound ^^
         get_a ^^
         compile_comparison I64Op.LeS ^^
         get_a ^^ compile_unboxed_const upper_bound ^^
         compile_comparison I64Op.LtS ^^
         G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
         get_res ^^
         compile_comparison I64Op.Eq ^^
         E.else_trap_with env ("fit_signed_bits failure_"^Int.to_string n) ^^
         get_res)
     else G.nop

  let fits_signed_bits env n =
    let set_a, get_a = new_local env "a" in
    try_unbox I64Type (fun _ -> match n with
        | 64 -> G.i Drop ^^ Bool.lit true
        | 8 | 16 | 32 ->
           set_a ^^
           get_a ^^ get_a ^^ compile_shrS_const 1L ^^
           G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^
           compile_bitand_const
             Int64.(shift_left minus_one ((n-1) + (64 - BitTagged.ubits_of Type.Int))) ^^
           compile_test I64Op.Eqz ^^
           sanity_check_fits_signed_bits env n get_a
        | _ -> assert false
      )
      (fun env -> Num.fits_signed_bits env n)
      env

  let compile_abs env =
    let sminl = Int64.shift_left 1L (BitTagged.sbits_of Type.Int) in
    let sminl_shifted = Int64.shift_left sminl (64 - BitTagged.ubits_of Type.Int) in
    try_unbox I64Type
      begin
        fun _ ->
        let set_a, get_a = new_local env "a" in
        clear_tag env ^^
        set_a ^^
        get_a ^^ compile_unboxed_const 0L ^^ compile_comparison I64Op.LtS ^^
        E.if1 I64Type
          begin
            get_a ^^
            (* -2^sbits is small enough for compact representation, but 2^sbits isn't *)
            compile_eq_const sminl_shifted ^^ (* i.e. -2^sbits shifted *)
            E.if1 I64Type
              (compile_unboxed_const sminl ^^ Num.from_word64 env)
              begin
                (* absolute value works directly on shifted representation *)
                compile_unboxed_const 0L ^^
                get_a ^^
                G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
                apply_tag env
              end
          end
          begin
            get_a ^^
            compile_bitor_const (TaggingScheme.tag_of_typ Type.Int)
          end
      end
      Num.compile_abs
      env

  let compile_load_from_word64 env get_data_buf = function
    | false -> get_data_buf ^^ E.call_import env "rts" "bigint_leb128_decode_word64"
    | true -> get_data_buf ^^ E.call_import env "rts" "bigint_sleb128_decode_word64"

  let compile_load_from_data_buf env get_data_buf signed =
    (* see Note [speculating for short (S)LEB encoded bignums] *)
    ReadBuf.speculative_read_word64 env get_data_buf ^^
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    compile_xor_const (-1L) ^^
    compile_bitand_const 0x8080_8080_8080_8080L ^^
    let set_eom, get_eom = new_local env "eom" in
    set_eom ^^ get_eom ^^
    compile_test I64Op.Eqz ^^
    E.if1 I64Type
      begin
        Num.compile_load_from_data_buf env get_data_buf signed
      end
      begin
        get_a ^^
        get_eom ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Ctz)) ^^
        compile_load_from_word64 env get_data_buf signed
      end

  let compile_store_to_data_buf_unsigned env =
    let set_x, get_x = new_local env "x" in
    let set_buf, get_buf = new_local env "buf" in
    set_x ^^ set_buf ^^
    get_x ^^
    try_unbox I64Type
      (fun env ->
        BitTagged.untag __LINE__ env Type.Int ^^ set_x ^^
        I32Leb.compile_store_to_data_buf_unsigned env get_x get_buf
      )
      (fun env ->
        G.i Drop ^^
        get_buf ^^ get_x ^^ Num.compile_store_to_data_buf_unsigned env)
      env

  let compile_store_to_data_buf_signed env =
    let set_x, get_x = new_local env "x" in
    let set_buf, get_buf = new_local env "buf" in
    set_x ^^ set_buf ^^
    get_x ^^
    try_unbox I64Type
      (fun env ->
        BitTagged.untag __LINE__ env Type.Int ^^ set_x ^^
        I32Leb.compile_store_to_data_buf_signed env get_x get_buf
      )
      (fun env ->
        G.i Drop ^^
        get_buf ^^ get_x ^^ Num.compile_store_to_data_buf_signed env)
      env

  let compile_data_size_unsigned env =
    try_unbox I64Type
      (fun _ ->
        let set_x, get_x = new_local env "x" in
        BitTagged.untag __LINE__ env Type.Int ^^ set_x ^^
        I32Leb.compile_leb128_size get_x
      )
      (fun env -> Num.compile_data_size_unsigned env)
      env

  let compile_data_size_signed env =
    try_unbox I64Type
      (fun _ ->
        let set_x, get_x = new_local env "x" in
        BitTagged.untag __LINE__ env Type.Int ^^ set_x ^^
        I32Leb.compile_sleb128_size get_x
      )
      (fun env -> Num.compile_data_size_signed env)
      env

  let from_signed_word64 env =
    let set_a, get_a = new_local env "a" in
    set_a ^^
    get_a ^^ BitTagged.if_can_tag_signed env Type.Int [I64Type]
      (get_a ^^ BitTagged.tag env Type.Int)
      (get_a ^^ Num.from_signed_word64 env)

  let from_signed_word_compact env =
    begin
      if TaggingScheme.debug || !(Flags.sanity)
      then
      let set_a, get_a = new_local env "a" in
      set_a ^^
      get_a ^^ BitTagged.if_can_tag_signed env Type.Int [I64Type]
        get_a
        (E.trap_with env "from_signed_word_compact")
      else G.nop
    end ^^
    BitTagged.tag env Type.Int

  let from_word64 env =
    let set_a, get_a = new_local env "a" in
    set_a ^^
    get_a ^^ BitTagged.if_can_tag_unsigned env Type.Int [I64Type]
      (get_a ^^ BitTagged.tag env Type.Int)
      (get_a ^^ Num.from_word64 env)

  let truncate_to_word64 env =
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    BitTagged.if_tagged_scalar env [I64Type]
      (get_a ^^ BitTagged.untag __LINE__ env Type.Int)
      (get_a ^^ Num.truncate_to_word64 env)

  let truncate_to_word32 env =
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    BitTagged.if_tagged_scalar env [I64Type]
      (get_a ^^ BitTagged.untag __LINE__ env Type.Int)
      (get_a ^^ Num.truncate_to_word32 env)

  let to_word64 env =
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    BitTagged.if_tagged_scalar env [I64Type]
      (get_a ^^ BitTagged.untag __LINE__ env Type.Int)
      (get_a ^^ Num.to_word64 env)

  let to_word64_with env get_err_msg =
    let set_a, get_a = new_local env "a" in
    set_a ^^ get_a ^^
    BitTagged.if_tagged_scalar env [I64Type]
      (get_a ^^ BitTagged.untag __LINE__ env Type.Int)
      (get_a ^^ Num.to_word64_with env get_err_msg)
end

module BigNumLibtommath : BigNumType = struct

  let to_word64 env = E.call_import env "rts" "bigint_to_word64_trap"
  let to_word64_with env get_err_msg = get_err_msg ^^ E.call_import env "rts" "bigint_to_word64_trap_with"

  let truncate_to_word32 env = E.call_import env "rts" "bigint_to_word32_wrap" ^^ G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
  let truncate_to_word64 env = E.call_import env "rts" "bigint_to_word64_wrap"

  let from_signed_word_compact env = E.call_import env "rts" "bigint_of_int64"
  let from_word64 env = E.call_import env "rts" "bigint_of_word64"
  let from_signed_word64 env = E.call_import env "rts" "bigint_of_int64"

  let compile_data_size_unsigned env = E.call_import env "rts" "bigint_leb128_size"
  let compile_data_size_signed env = E.call_import env "rts" "bigint_sleb128_size"

  let compile_store_to_data_buf_unsigned env =
    let (set_buf, get_buf) = new_local env "buf" in
    let (set_n, get_n) = new_local env "n" in
    set_n ^^ set_buf ^^
    get_n ^^ get_buf ^^ E.call_import env "rts" "bigint_leb128_encode" ^^
    get_n ^^ E.call_import env "rts" "bigint_leb128_size"

  let compile_store_to_data_buf_signed env =
    let (set_buf, get_buf) = new_local env "buf" in
    let (set_n, get_n) = new_local env "n" in
    set_n ^^ set_buf ^^
    get_n ^^ get_buf ^^ E.call_import env "rts" "bigint_sleb128_encode" ^^
    get_n ^^ E.call_import env "rts" "bigint_sleb128_size"

  let compile_load_from_data_buf env get_data_buf = function
    | false -> get_data_buf ^^ E.call_import env "rts" "bigint_leb128_decode"
    | true -> get_data_buf ^^ E.call_import env "rts" "bigint_sleb128_decode"

  let constant env n =
    (* See enum mp_sign *)
    let sign = if Big_int.sign_big_int n >= 0 then 0l else 1l in

    let n = Big_int.abs_big_int n in

    let limbs =
      (* see MP_DIGIT_BIT for MP_64BIT *)
      let twoto60 = Big_int.power_int_positive_int 2 60 in
      let rec go n =
        if Big_int.sign_big_int n = 0
        then []
        else
          let (a, b) = Big_int.quomod_big_int n twoto60 in
          StaticBytes.[ I64 (Big_int.int64_of_big_int b) ] @ go a
      in go n
    in

    (* how many 64 bit digits *)
    let size = Int32.of_int (List.length limbs) in

    (* cf. mp_int in tommath.h *)
    (* Tom's math library is compiled with 64-bit `mp_digit` size. *)
    let payload = StaticBytes.[
      I32 size; (* used *)
      I32 size; (* size; relying on Heap.word_size == size_of(mp_digit) *)
      I32 sign;
      I32 0l; (* padding because of 64-bit alignment of subsequent pointer *)
      I64 0L; (* dp; this will be patched in BigInt::mp_int_ptr in the RTS when used *)
    ] @ limbs
    in

    Tagged.shared_object __LINE__ env (fun env ->
      (*TODO: Why isn't this just loaded from a segment? *)
      let instructions =
        let words = StaticBytes.as_words payload in
        List.map compile_unboxed_const words in
      Tagged.obj env Tagged.BigInt instructions
    )

  let assert_nonneg env =
    Func.share_code1 Func.Never env "assert_nonneg" ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^
      E.call_import env "rts" "bigint_isneg" ^^ Bool.from_rts_int32 ^^
      E.then_trap_with env "Natural subtraction underflow" ^^
      get_n
    )

  let compile_abs env = E.call_import env "rts" "bigint_abs"
  let compile_neg env = E.call_import env "rts" "bigint_neg"
  let compile_add env = E.call_import env "rts" "bigint_add"
  let compile_mul env = E.call_import env "rts" "bigint_mul"
  let compile_signed_sub env = E.call_import env "rts" "bigint_sub"
  let compile_signed_div env = E.call_import env "rts" "bigint_div"
  let compile_signed_mod env = E.call_import env "rts" "bigint_rem"
  let compile_unsigned_sub env = E.call_import env "rts" "bigint_sub" ^^ assert_nonneg env
  let compile_unsigned_rem env = E.call_import env "rts" "bigint_rem"
  let compile_unsigned_div env = E.call_import env "rts" "bigint_div"
  let compile_unsigned_pow env = E.call_import env "rts" "bigint_pow"
  let compile_lsh env = E.call_import env "rts" "bigint_lsh"
  let compile_rsh env = E.call_import env "rts" "bigint_rsh"

  let compile_eq env = E.call_import env "rts" "bigint_eq" ^^ Bool.from_rts_int32
  let compile_is_negative env = E.call_import env "rts" "bigint_isneg" ^^ Bool.from_rts_int32
  let compile_relop env = function
      | Lt -> E.call_import env "rts" "bigint_lt" ^^ Bool.from_rts_int32
      | Le -> E.call_import env "rts" "bigint_le" ^^ Bool.from_rts_int32
      | Ge -> E.call_import env "rts" "bigint_ge" ^^ Bool.from_rts_int32
      | Gt -> E.call_import env "rts" "bigint_gt" ^^ Bool.from_rts_int32

  let fits_signed_bits env bits =
    E.call_import env "rts" "bigint_2complement_bits" ^^
    compile_unboxed_const (Int64.of_int bits) ^^
    compile_comparison I64Op.LeU
  let fits_unsigned_bits env bits =
    E.call_import env "rts" "bigint_count_bits" ^^
    compile_unboxed_const (Int64.of_int bits) ^^
    compile_comparison I64Op.LeU

end (* BigNumLibtommath *)

module BigNum = MakeCompact(BigNumLibtommath)

(* Primitive functions *)
module Prim = struct
  (* The {Nat,Int}{8,16,32} bits sit in the MSBs of the i64, in this manner
     we can perform almost all operations, with the exception of
     - Mul (needs shr of one operand)
     - Shr (needs masking of result)
     - Rot (needs duplication into LSBs, masking of amount and masking of result)
     - ctz (needs shr of operand or sub from result)

     Both {Nat,Int}{8,16,32} fit into the vanilla stackrep, so no boxing is necessary.
     This MSB-stored schema is also essentially what the interpreter is using.
  *)
  let prim_word64toNat = BigNum.from_word64
  let prim_shiftWordNtoUnsigned env b =
    compile_shrU_const b ^^
    prim_word64toNat env
  let prim_word64toInt = BigNum.from_signed_word64
  let prim_shiftWordNtoSigned env b =
    compile_shrS_const b ^^
    prim_word64toInt env
  let prim_intToWord64 = BigNum.truncate_to_word64
  let prim_intToWordNShifted env b =
    prim_intToWord64 env ^^
    TaggedSmallWord.shift_leftWordNtoI64 b
end (* Prim *)

module Blob = struct
  (* The layout of a blob object is

     ┌──────┬─────┬─────────┬──────────────────┐
     │ obj header │ n_bytes │ bytes (padded) … │
     └──────┴─────┴─────────┴──────────────────┘

    The object header includes the object tag (Blob) and the forwarding pointer.

    This heap object is used for various kinds of binary, non-pointer data.

    When used for Text values, the bytes are UTF-8 encoded code points from
    Unicode.
  *)

  let header_size = Int64.add Tagged.header_size 1L
  let len_field = Int64.add Tagged.header_size 0L

  let len env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env len_field

  let len_nat env =
    Func.share_code1 Func.Never env "blob_len" ("text", I64Type) [I64Type] (fun env get ->
      get ^^
      len env ^^
      BigNum.from_word64 env
    )

  let alloc env sort len =
    compile_unboxed_const Tagged.(int_of_tag (Blob sort)) ^^
    len ^^
    E.call_import env "rts" "alloc_blob" ^^
    (* uninitialized blob payload is allowed by the barrier *)
    Tagged.allocation_barrier env

  let unskewed_payload_offset env = Int64.(add ptr_unskew (mul Heap.word_size header_size))

  let payload_ptr_unskewed env =
    Tagged.load_forwarding_pointer env ^^
    compile_add_const (unskewed_payload_offset env)

  let load_data_segment env sort segment_index data_length =
    let (set_blob, get_blob) = new_local env "data_segment_blob" in
    alloc env sort data_length ^^ set_blob ^^
    get_blob ^^ payload_ptr_unskewed env ^^ (* target address *)
    compile_const_32 0l ^^ (* data offset *)
    data_length ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    G.i (MemoryInit (nr segment_index)) ^^
    get_blob

  let constant env sort payload : E.shared_value =
    Tagged.shared_object __LINE__ env (fun env ->
      let blob_length = Int64.of_int (String.length payload) in
      let segment_index = E.add_static env StaticBytes.[Bytes payload] in
      load_data_segment env sort segment_index (compile_unboxed_const blob_length)
    )

  let lit env sort payload =
    let shared_value = match sort with
      | Tagged.B | Tagged.T ->
        let bytes = payload in
        let value = Const.Lit (if sort = Tagged.B then Const.Blob bytes else Const.Text bytes) in
        E.constant_pool_add env value (fun () -> constant env sort bytes)
      | Tagged.P | Tagged.A ->
        (* TODO: perhaps extend Const.v for principal and actor blobs for sharing *)
        constant env sort payload
    in
    Tagged.materialize_shared_value env shared_value

  let as_ptr_len env = Func.share_code1 Func.Never env "as_ptr_size" ("x", I64Type) [I64Type; I64Type] (
    fun env get_x ->
      get_x ^^ payload_ptr_unskewed env ^^
      get_x ^^ len env
    )

  let lit_ptr_len env sort s =
    lit env sort s ^^
    as_ptr_len env

  let load_data_segment env sort segment_index data_length =
    let (set_blob, get_blob) = new_local env "data_segment_blob" in
    alloc env sort data_length ^^ set_blob ^^
    get_blob ^^ payload_ptr_unskewed env ^^ (* target address *)
    compile_const_32 0l ^^ (* data offset *)
    data_length ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    G.i (MemoryInit (nr segment_index)) ^^
    get_blob

  let of_ptr_size env = Func.share_code2 Func.Always env "blob_of_ptr_size" (("ptr", I64Type), ("size" , I64Type)) [I64Type] (
    fun env get_ptr get_size ->
      let (set_x, get_x) = new_local env "x" in
      alloc env Tagged.B get_size ^^ set_x ^^
      get_x ^^ payload_ptr_unskewed env ^^
      get_ptr ^^
      get_size ^^
      Heap.memcpy env ^^
      get_x
    )

  let copy env src_sort dst_sort =
    let name = Printf.sprintf "blob_copy_%s_%s"
                 (Int64.to_string (Tagged.int_of_tag (Tagged.Blob src_sort)))
                 (Int64.to_string (Tagged.int_of_tag (Tagged.Blob dst_sort)))
    in
    Func.share_code1 Func.Never env name ("src", I64Type) [I64Type] (
      fun env get_src ->
       let (set_dst, get_dst) = new_local env "dst" in
       alloc env dst_sort (get_src ^^ len env) ^^ set_dst ^^
       get_dst ^^ payload_ptr_unskewed env ^^
       get_src ^^ Tagged.sanity_check_tag __LINE__ env (Tagged.Blob src_sort) ^^
       as_ptr_len env ^^
       Heap.memcpy env ^^
       get_dst
    )

  let of_size_copy env sort get_size_fun copy_fun offset_fun =
    let (set_len, get_len) = new_local env "len" in
    let (set_blob, get_blob) = new_local env "blob" in
    get_size_fun env ^^ set_len ^^

    alloc env sort get_len ^^ set_blob ^^
    get_blob ^^ payload_ptr_unskewed env ^^
    offset_fun env ^^
    get_len ^^
    copy_fun env ^^

    get_blob

  (* Lexicographic blob comparison. Expects two blobs on the stack.
     Either specialized to a specific comparison operator, and returns a boolean,
     or implements the generic comparison, returning -1, 0 or 1 as Int64.
  *)
  let rec compare env op =
    (* return convention for the generic comparison function *)
    let is_lt = compile_unboxed_const (-1L) in
    let is_gt = compile_unboxed_one in
    let is_eq = compile_unboxed_const 0L in
    let open Operator in
    let name = match op with
        | Some LtOp -> "Blob.compare_lt"
        | Some LeOp -> "Blob.compare_le"
        | Some GeOp -> "Blob.compare_ge"
        | Some GtOp -> "Blob.compare_gt"
        | Some EqOp -> "Blob.compare_eq"
        | Some NeqOp -> "Blob.compare_neq"
        | None -> "Blob.compare" in
    Func.share_code2 Func.Always env name (("x", I64Type), ("y", I64Type)) [I64Type] (fun env get_x get_y ->
      match op with
        (* Some operators can be reduced to the negation of other operators *)
        | Some LtOp -> get_x ^^ get_y ^^ compare env (Some GeOp) ^^ Bool.neg
        | Some GtOp -> get_x ^^ get_y ^^ compare env (Some LeOp) ^^ Bool.neg
        | Some NeqOp -> get_x ^^ get_y ^^ compare env (Some EqOp) ^^ Bool.neg
        | _ ->
      begin
        let set_x = G.setter_for get_x in
        let set_y = G.setter_for get_y in
        get_x ^^ Tagged.load_forwarding_pointer env ^^ set_x ^^
        get_y ^^ Tagged.load_forwarding_pointer env ^^ set_y ^^

        let (set_len1, get_len1) = new_local env "len1" in
        let (set_len2, get_len2) = new_local env "len2" in
        let (set_len, get_len) = new_local env "len" in
        let (set_a, get_a) = new_local env "a" in
        let (set_b, get_b) = new_local env "b" in

        get_x ^^ len env ^^ set_len1 ^^
        get_y ^^ len env ^^ set_len2 ^^

        (* Find minimum length *)
        begin if op = Some EqOp then
          (* Early exit for equality *)
          get_len1 ^^ get_len2 ^^ compile_comparison I64Op.Eq ^^
          E.if0 G.nop (Bool.lit false ^^ G.i Return) ^^

          get_len1 ^^ set_len
        else
          get_len1 ^^ get_len2 ^^ compile_comparison I64Op.LeU ^^
          E.if0
            (get_len1 ^^ set_len)
            (get_len2 ^^ set_len)
        end ^^

        (* We could do word-wise comparisons if we know that the trailing bytes
           are zeroed *)
        get_len ^^
        from_0_to_n env (fun get_i ->
          get_x ^^
          payload_ptr_unskewed env ^^
          get_i ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
          G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack8, ZX)}) ^^
          set_a ^^

          get_y ^^
          payload_ptr_unskewed env ^^
          get_i ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
          G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack8, ZX)}) ^^
          set_b ^^

          get_a ^^ get_b ^^ compile_comparison I64Op.Eq ^^
          E.if0 G.nop (
            (* first non-equal elements *)
            begin match op with
            | Some LeOp -> get_a ^^ get_b ^^ compile_comparison I64Op.LeU
            | Some GeOp -> get_a ^^ get_b ^^ compile_comparison I64Op.GeU
            | Some EqOp -> Bool.lit false
            | None -> get_a ^^ get_b ^^ compile_comparison I64Op.LtU ^^
                      E.if1 I64Type is_lt is_gt
            | _ -> assert false
            end ^^
            G.i Return
          )
        ) ^^
        (* Common prefix is same *)
        match op with
        | Some LeOp -> get_len1 ^^ get_len2 ^^ compile_comparison I64Op.LeU
        | Some GeOp -> get_len1 ^^ get_len2 ^^ compile_comparison I64Op.GeU
        | Some EqOp -> Bool.lit true (* NB: Different length handled above *)
        | None ->
            get_len1 ^^ get_len2 ^^ compile_comparison I64Op.LtU ^^
            E.if1 I64Type is_lt (
              get_len1 ^^ get_len2 ^^ compile_comparison I64Op.GtU ^^
              E.if1 I64Type is_gt is_eq
            )
        | _ -> assert false
      end
  )

  let iter env =
    E.call_import env "rts" "blob_iter"
  let iter_done env =
    E.call_import env "rts" "blob_iter_done"
  let iter_next env =
    E.call_import env "rts" "blob_iter_next" ^^
    TaggedSmallWord.msb_adjust Type.Nat8

  (* Dynamic blob index access. Returns the value of the element.
     Does bounds checking *)
  let idx env =
    Func.share_code2 Func.Never env "Blob.idx" (("blob", I64Type), ("idx", I64Type)) [I64Type] (fun env get_blob get_idx ->
      (* No need to check the lower bound, we interpret idx as unsigned *)
      (* Check the upper bound *)
      get_idx ^^
      get_blob ^^ len env ^^
      compile_comparison I64Op.LtU ^^
      E.else_trap_with env "Blob index out of bounds" ^^

      get_idx ^^
      compile_add_const Int64.(mul header_size Heap.word_size |> add ptr_unskew) ^^
      get_blob ^^
      Tagged.load_forwarding_pointer env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
      G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = Some (Pack8, ZX)}) ^^
      TaggedSmallWord.msb_adjust Type.Nat8 ^^
      TaggedSmallWord.tag env Type.Nat8
    )

  (* As above, but taking a bigint (Nat), and reporting overflow as out of bounds *)
  let idx_bigint env =
    Func.share_code2 Func.Never env "Blob.idx_bigint" (("blob", I64Type), ("idx", I64Type)) [I64Type] (fun env get_blob get_idx ->
      get_blob ^^
      get_idx ^^
      BigNum.to_word64_with env (lit env Tagged.T "Blob index out of bounds") ^^
      idx env
  )

  let dyn_alloc_scratch env =
    let (set_len, get_len) = new_local env "len" in
    set_len ^^
    alloc env Tagged.B get_len ^^ payload_ptr_unskewed env

end (* Blob *)

module Object = struct
  (* An object with a mutable field1 and immutable field 2 has the following
     heap layout:

     ┌──────┬─────┬──────────┬─────────┬─────────────┬───┐
     │ obj header │ hash_ptr │ ind_ptr │ field2_data │ … │
     └──────┴─────┴┬─────────┴┬────────┴─────────────┴───┘
          ┌────────┘          │
          │   ┌───────────────┘
          │   ↓
          │  ╶─┬────────┬─────────────┐
          │    │ MutBox │ field1_data │
          ↓    └────────┴─────────────┘
          ┌─────────────┬─────────────┬─────────────┬───┐
          │ blob header │ field1_hash │ field2_hash │ … │
          └─────────────┴─────────────┴─────────────┴───┘

     The object header includes the object tag (Object) and the forwarding pointer.
     The size of the object (number of fields) can be derived from the hash blob via `hash_ptr`.

     The field hashes reside in a blob inside the dynamic heap.
     The hash blob needs to be tracked by the GC, but not the content of the hash blob.
     This is because the hash values are plain numbers that would look like skewed pointers.ters.
     The hash_ptr is skewed.

     The field2_data for immutable fields is a vanilla word.

     The field1_data for mutable fields are pointers to a MutBox. This indirection
     is a consequence of how we compile object literals with `await` instructions,
     as these mutable fields need to be able to alias local mutable variables, e.g.
     `{ public let f = 1; await async (); public let var v = 2}`.
     Other use cases are object constructors with public and private mutable fields,
     where the physical record only wraps the public fields.
     Moreover, closures can selectively capture the individual fields instead of
     the containing object.
     Finally, classical Candid stabilization/destabilization also relies on the
     indirection of mutable fields, to reserve and store alias information in those
     locations.

     We could alternatively switch to an allocate-first approach in the
     await-translation of objects, and get rid of this indirection -- if it were
     not for the implementing of sharing of mutable stable values.
   *)

  let header_size = Int64.add Tagged.header_size 1L

  let hash_ptr_field = Int64.add Tagged.header_size 0L

  module FieldEnv = Env.Make(String)

  (* This is for non-recursive objects. *)
  (* The instructions in the field already create the indirection if needed *)
  let object_builder env (fs : (string * (E.t -> G.t)) list ) =
    let name_pos_map =
      fs |>
        (* We could store only public fields in the object, but
          then we need to allocate separate boxes for the non-public ones:
          List.filter (fun (_, vis, f) -> vis.it = Public) |>
        *)
        List.map (fun (n,_) -> (E.hash env n, n)) |>
        List.sort compare |>
        List.mapi (fun i (_h,n) -> (n,Int64.of_int i)) |>
        List.fold_left (fun m (n,i) -> FieldEnv.add n i m) FieldEnv.empty in

      let sz = Int64.of_int (FieldEnv.cardinal name_pos_map) in

      (* Create hash blob *)
      let hashes = fs |>
        List.map (fun (n,_) -> E.hash env n) |>
        List.sort compare in
      let hash_blob =
        let hash_payload = StaticBytes.[ i64s hashes ] in
        (* NB: Blob.lit shares blobs *)
        Blob.lit env Tagged.B (StaticBytes.as_bytes hash_payload)
      in
      (fun env ->
        (* Allocate memory *)
        let (set_ri, get_ri, ri) = new_local_ env I64Type "obj" in
        Tagged.alloc env (Int64.add header_size sz) Tagged.Object ^^
        set_ri ^^

        (* Set hash_ptr *)
        get_ri ^^
        hash_blob ^^
        Tagged.store_field env hash_ptr_field ^^

        (* Write all the fields *)
        let init_field (name, generate_value) : G.t =
          (* Write the pointer to the indirection *)
          get_ri ^^
          generate_value env ^^
          let i = FieldEnv.find name name_pos_map in
          let offset = Int64.add header_size i in
          Tagged.store_field env offset
        in
        G.concat_map init_field fs ^^

        (* Return the pointer to the object *)
        get_ri ^^
        Tagged.allocation_barrier env
      )

  let constant env (fs : (string * E.shared_value) list) =
    let materialize_fields = List.map (fun (name, value) -> (name, fun env -> Tagged.materialize_shared_value env value)) fs in
    let allocation = object_builder env materialize_fields in
    Tagged.shared_object __LINE__ env allocation

  (* This is for non-recursive objects, i.e. ObjNewE *)
  (* The instructions in the field already create the indirection if needed *)
  let lit_raw env (fs : (string * (unit -> G.t)) list ) =
    let materialize_fields = List.map (fun (name, generate_value) -> (name, (fun env -> generate_value ()))) fs in
    let allocation = object_builder env materialize_fields in
    allocation env

  (* Reflection used by orthogonal persistence:
     Check whether an (actor) object contains a specific field *)
  let contains_field env field =
    compile_unboxed_const (E.hash env field) ^^
    E.call_import env "rts" "contains_field" ^^
    Bool.from_rts_int32

  (* Returns a pointer to the object field (without following the field indirection) *)
  let idx_hash_raw env low_bound =
    let name = Printf.sprintf "obj_idx<%d>" low_bound  in
    Func.share_code2 Func.Always env name (("x", I64Type), ("hash", I64Type)) [I64Type] (fun env get_x get_hash ->
      let set_x = G.setter_for get_x in
      let set_h_ptr, get_h_ptr = new_local env "h_ptr" in

      get_x ^^ Tagged.load_forwarding_pointer env ^^ set_x ^^

      get_x ^^ Tagged.load_field env hash_ptr_field ^^
      Blob.payload_ptr_unskewed env ^^

      (* Linearly scan through the fields (binary search can come later) *)
      (* unskew h_ptr and advance both to low bound *)
      compile_add_const Int64.(mul Heap.word_size (of_int low_bound)) ^^
      set_h_ptr ^^
      get_x ^^
      compile_add_const Int64.(mul Heap.word_size (add header_size (of_int low_bound))) ^^
      set_x ^^
      G.loop0 (
          get_h_ptr ^^ load_unskewed_ptr ^^
          get_hash ^^ compile_comparison I64Op.Eq ^^
          E.if0
            (get_x ^^ G.i Return)
            (get_h_ptr ^^ compile_add_const Heap.word_size ^^ set_h_ptr ^^
            get_x ^^ compile_add_const Heap.word_size ^^ set_x ^^
            G.i (Br (nr 1l)))
        ) ^^
      G.i Unreachable
    )

  (* Returns a pointer to the object field (possibly following the indirection) *)
  let idx_hash env low_bound indirect =
    if indirect
    then
      let name = Printf.sprintf "obj_idx_ind<%d>" low_bound in
      Func.share_code2 Func.Never env name (("x", I64Type), ("hash", I64Type)) [I64Type] (fun env get_x get_hash ->
      get_x ^^ get_hash ^^
      idx_hash_raw env low_bound ^^
      load_ptr ^^ Tagged.load_forwarding_pointer env ^^
      compile_add_const (Int64.mul MutBox.field Heap.word_size)
    )
    else idx_hash_raw env low_bound

  let field_type env obj_type s =
    let _, fields = Type.as_obj_sub [s] obj_type in
    Type.lookup_val_field s fields

  (* Determines whether the field is mutable (and thus needs an indirection) *)
  let is_mut_field env obj_type s =
    let _, fields = Type.as_obj_sub [s] obj_type in
    Type.is_mut (Type.lookup_val_field s fields)

  (* Computes a lower bound for the positional index of a field in an object *)
  let field_lower_bound env obj_type s =
    let open Type in
    let _, fields = as_obj_sub [s] obj_type in
    List.iter (function {typ = Typ _; _} -> assert false | _ -> ()) fields;
    let sorted_by_hash =
      List.sort
        (fun (h1, _) (h2, _) -> compare_uint64 h1 h2)
        (List.map (fun f -> E.hash env f.lab, f) fields) in
    match Lib.List.index_of s (List.map (fun (_, {lab; _}) -> lab) sorted_by_hash) with
    | Some i -> i
    | _ -> assert false

  (* Returns a pointer to the object field (without following the indirection) *)
  let idx_raw env f =
    compile_unboxed_const (E.hash env f) ^^
    idx_hash_raw env 0

  (* Returns a pointer to the object field (possibly following the indirection) *)
  let idx env obj_type f =
    compile_unboxed_const (E.hash env f) ^^
    idx_hash env (field_lower_bound env obj_type f) (is_mut_field env obj_type f)

  (* load the value (or the mutbox) *)
  let load_idx_raw env f =
    idx_raw env f ^^
    load_ptr

  (* load the actual value (dereferencing the mutbox) *)
  let load_idx env obj_type f =
    idx env obj_type f ^^
    load_ptr

end (* Object *)

module Region = struct
  (*
    See rts/motoko-rts/src/region.rs
   *)

  (* Object layout:

     ┌─────┬──────────┬──────────────────┬─────────────────┐
     │ tag │ id_field │ page_count_field │ vec_pages_field │
     └─────┴──────────┴──────────────────┴─────────────────┘
            (unboxed, low 16 bits, rest 0-initialized padding)
                        unboxed u32
                                          Blob
  *)

  let alloc_region env =
    E.call_import env "rts" "alloc_region"

  let init_region env =
    E.call_import env "rts" "init_region"

  (* field accessors *)
  (* NB: all these opns must resolve forwarding pointers here or in RTS *)
  let id env =
    E.call_import env "rts" "region_id"

  let page_count env =
    E.call_import env "rts" "region_page_count"

  let vec_pages env =
    E.call_import env "rts" "region_vec_pages"

  let new_ env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_new"

  let size env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_size"

  let grow env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_grow"

  let load_blob env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_blob"
  let store_blob env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_blob"

  let load_word8 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_word8"
  let store_word8 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_word8"

  let load_word16 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_word16"
  let store_word16 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_word16"

  let load_word32 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_word32"
  let store_word32 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_word32"

  let load_word64 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_word64"
  let store_word64 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_word64"

  let load_float64 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_load_float64"
  let store_float64 env =
    E.require_stable_memory env;
    E.call_import env "rts" "region_store_float64"

end

module Text = struct
  (*
  Most of the heavy lifting around text values is in rts/motoko-rts/src/text.rs
  *)

  (* The layout of a concatenation node is

     ┌──────┬─────┬─────────┬───────┬───────┐
     │ obj header │ n_bytes │ text1 │ text2 │
     └──────┴─────┴─────────┴───────┴───────┘

    The object header includes the object tag (TAG_CONCAT defined in rts/types.rs) and the forwarding pointer

    This is internal to rts/text.c, with the exception of GC-related code.
  *)

  let of_ptr_size env =
    E.call_import env "rts" "text_of_ptr_size"
  let concat env =
    E.call_import env "rts" "text_concat"
  let size env =
    E.call_import env "rts" "text_size"
  let to_buf env =
    E.call_import env "rts" "text_to_buf"
  let len_nat env =
    Func.share_code1 Func.Never env "text_len" ("text", I64Type) [I64Type] (fun env get ->
      get ^^
      E.call_import env "rts" "text_len" ^^
      BigNum.from_word64 env
    )
  let prim_showChar env =
    TaggedSmallWord.lsb_adjust_codepoint env ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    E.call_import env "rts" "text_singleton"
  let to_blob env = E.call_import env "rts" "blob_of_text"

  let lowercase env = E.call_import env "rts" "text_lowercase"
  let uppercase env = E.call_import env "rts" "text_uppercase"

  let of_blob env =
    let (set_blob, get_blob) = new_local env "blob" in
    set_blob ^^
    get_blob ^^ Blob.as_ptr_len env ^^
    E.call_import env "rts" "utf8_valid" ^^
    Bool.from_rts_int32 ^^
    E.if1 I64Type
      (get_blob ^^ Blob.copy env Tagged.B Tagged.T ^^
       set_blob ^^
       Opt.inject_simple env get_blob)
      (Opt.null_lit env)

  let iter env =
    E.call_import env "rts" "text_iter"
  let iter_done env =
    E.call_import env "rts" "text_iter_done"
  let iter_next env =
    E.call_import env "rts" "text_iter_next" ^^ Bool.from_rts_int32 ^^
    TaggedSmallWord.msb_adjust_codepoint

  let compare env op =
    let open Operator in
    let name = match op with
        | LtOp -> "Text.compare_lt"
        | LeOp -> "Text.compare_le"
        | GeOp -> "Text.compare_ge"
        | GtOp -> "Text.compare_gt"
        | EqOp -> "Text.compare_eq"
        | NeqOp -> assert false in
    Func.share_code2 Func.Never env name (("x", I64Type), ("y", I64Type)) [I64Type] (fun env get_x get_y ->
      get_x ^^ Tagged.load_forwarding_pointer env ^^
      get_y ^^ Tagged.load_forwarding_pointer env ^^
      E.call_import env "rts" "text_compare" ^^
      compile_unboxed_const 0L ^^
      match op with
        | LtOp -> compile_comparison I64Op.LtS
        | LeOp -> compile_comparison I64Op.LeS
        | GtOp -> compile_comparison I64Op.GtS
        | GeOp -> compile_comparison I64Op.GeS
        | EqOp -> compile_comparison I64Op.Eq
        | NeqOp -> assert false
    )


end (* Text *)

module Arr = struct
  (* Object layout:

     ┌──────┬─────┬──────────┬────────┬───┐
     │ obj header │ n_fields │ field1 │ … │
     └──────┴─────┴──────────┴────────┴───┘

     The object  header includes the object tag (Array) and the forwarding pointer.

     No difference between mutable and immutable arrays.
  *)

  (* NB max_array_size must agree with limit 2^61 imposed by RTS alloc_array() *)
  let max_array_size = Int64.shift_left 1L 61 (* inclusive *)

  let header_size = Int64.add Tagged.header_size 1L
  let element_size = 8L
  let len_field = Int64.add Tagged.header_size 0L

  let len env =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env len_field

  (* Static array access. No checking *)
  let load_field env n =
    Tagged.load_forwarding_pointer env ^^
    Tagged.load_field env Int64.(add n header_size)

  (* Dynamic array access. Returns the address (not the value) of the field.
     Does no bounds checking *)
  let unsafe_idx env =
    Func.share_code2 Func.Never env "Array.unsafe_idx" (("array", I64Type), ("idx", I64Type)) [I64Type] (fun env get_array get_idx ->
      get_idx ^^
      compile_add_const header_size ^^
      compile_mul_const element_size ^^
      get_array ^^
      Tagged.load_forwarding_pointer env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
    )

  (* Dynamic array access. Returns the address (not the value) of the field.
     Does bounds checking *)
  let idx env =
    Func.share_code2 Func.Never env "Array.idx" (("array", I64Type), ("idx", I64Type)) [I64Type] (fun env get_array get_idx ->
      (* No need to check the lower bound, we interpret idx as unsigned *)
      (* Check the upper bound *)
      get_idx ^^
      get_array ^^ len env ^^
      compile_comparison I64Op.LtU ^^
      E.else_trap_with env "Array index out of bounds" ^^

      get_idx ^^
      compile_add_const header_size ^^
      compile_mul_const element_size ^^
      get_array ^^
      Tagged.load_forwarding_pointer env ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
    )

  (* As above, but taking a bigint (Nat), and reporting overflow as out of bounds *)
  let idx_bigint env =
    Func.share_code2 Func.Never env "Array.idx_bigint" (("array", I64Type), ("idx", I64Type)) [I64Type] (fun env get_array get_idx ->
      get_array ^^
      get_idx ^^
      BigNum.to_word64_with env (Blob.lit env Tagged.T "Array index out of bounds") ^^
      idx env
  )

  let element_type env typ = match Type.promote typ with
     | Type.Array element_type -> element_type
     | _ -> assert false

  (* Compile an array literal. *)
  let lit env sort element_instructions =
    Tagged.obj env Tagged.(Array sort)
     ([ compile_unboxed_const (Wasm.I64.of_int_u (List.length element_instructions))
      ] @ element_instructions)

  let constant env sort elements =
    Tagged.shared_object __LINE__ env (fun env ->
      let materialized_elements = List.map (fun element -> Tagged.materialize_shared_value env element) elements in
      lit env sort materialized_elements
    )

  (* Does not initialize the fields! *)
  (* Note: Post allocation barrier must be applied after initialization *)
  let alloc env array_sort len =
    compile_unboxed_const Tagged.(int_of_tag (Array array_sort)) ^^
    len ^^
    E.call_import env "rts" "alloc_array"

  let iterate env get_array body =
    let (set_boundary, get_boundary) = new_local env "boundary" in
    let (set_pointer, get_pointer) = new_local env "pointer" in
    let set_array = G.setter_for get_array in

    get_array ^^ Tagged.load_forwarding_pointer env ^^ set_array ^^

    (* Initial element pointer, skewed *)
    compile_unboxed_const header_size ^^
    compile_mul_const element_size ^^
    get_array ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    set_pointer ^^

    (* Upper pointer boundary, skewed *)
    get_array ^^
    Tagged.load_field env len_field ^^
    compile_mul_const element_size ^^
    get_pointer ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    set_boundary ^^

    (* Loop through all elements *)
    compile_while env
    ( get_pointer ^^
      get_boundary ^^
      compile_comparison I64Op.LtU
    ) (
      body get_pointer ^^

      (* Next element pointer, skewed *)
      get_pointer ^^
      compile_add_const element_size ^^
      set_pointer
    )

  (* The primitive operations *)
  (* No need to wrap them in RTS functions: They occur only once, in the prelude. *)
  let init env =
    let (set_x, get_x) = new_local env "x" in
    let (set_r, get_r) = new_local env "r" in
    set_x ^^

    (* Allocate *)
    BigNum.to_word64 env ^^
    set_r ^^
    alloc env Tagged.M get_r ^^
    set_r ^^

    (* Write elements *)
    iterate env get_r (fun get_pointer ->
      get_pointer ^^
      get_x ^^
      store_ptr
    ) ^^

    get_r ^^
    Tagged.allocation_barrier env


  let tabulate env sort =
    let (set_f, get_f) = new_local env "f" in
    let (set_r, get_r) = new_local env "r" in
    let (set_i, get_i) = new_local env "i" in
    set_f ^^

    (* Allocate *)
    BigNum.to_word64 env ^^
    set_r ^^
    alloc env sort get_r ^^
    set_r ^^

    (* Initial index *)
    compile_unboxed_const 0L ^^
    set_i ^^

    (* Write elements *)
    iterate env get_r (fun get_pointer ->
      get_pointer ^^
      (* The closure *)
      get_f ^^
      Closure.prepare_closure_call env ^^
      (* The arg *)
      get_i ^^
      BigNum.from_word64 env ^^
      (* The closure again *)
      get_f ^^
      (* Call *)
      Closure.call_closure env 1 1 ^^
      store_ptr ^^

      (* Increment index *)
      get_i ^^
      compile_add_const 1L ^^
      set_i
    ) ^^
    get_r ^^
    Tagged.allocation_barrier env

  let ofBlob env sort =
    let name = Tagged.(match sort with I -> "Arr.ofBlob" | M -> "Arr.ofBlobMut" | _ -> assert false) in
    Func.share_code1 Func.Always env name ("blob", I64Type) [I64Type] (fun env get_blob ->
      let (set_len, get_len) = new_local env "len" in
      let (set_r, get_r) = new_local env "r" in

      get_blob ^^ Blob.len env ^^ set_len ^^

      alloc env sort get_len ^^ set_r ^^

      get_len ^^ from_0_to_n env (fun get_i ->
        get_r ^^ get_i ^^ unsafe_idx env ^^
        get_blob ^^ Blob.payload_ptr_unskewed env ^^
        get_i ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
        G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack8, ZX)}) ^^
        TaggedSmallWord.msb_adjust Type.Nat8 ^^
        TaggedSmallWord.tag env Type.Nat8 ^^
        store_ptr
      ) ^^
      get_r ^^
      Tagged.allocation_barrier env
    )

  let toBlob env =
    Func.share_code1 Func.Always env "Arr.toBlob" ("array", I64Type) [I64Type] (fun env get_a ->
      let (set_len, get_len) = new_local env "len" in
      let (set_r, get_r) = new_local env "r" in

      get_a ^^ len env ^^ set_len ^^

      Blob.alloc env Tagged.B get_len ^^ set_r ^^

      get_len ^^ from_0_to_n env (fun get_i ->
        get_r ^^ Blob.payload_ptr_unskewed env ^^
        get_i ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
        get_a ^^ get_i ^^ unsafe_idx env ^^
        load_ptr ^^
        TaggedSmallWord.lsb_adjust Type.Nat8 ^^
        G.i (Store {ty = I64Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.Pack8})
      ) ^^

      get_r
    )

end (* Array *)

module Tuple = struct
  (* Tuples use the same object representation (and same tag) as arrays.
     Even though we know the size statically, we still need the size
     information for the GC.

     One could introduce tags for small tuples, to save one word.
  *)

  (* We represent the boxed empty tuple as the unboxed scalar 0, i.e. simply as
     number (but really anything is fine, we never look at this) *)
  let unit_vanilla_lit env = TaggingScheme.unit_tag  (* all tag, trivial payload *)
  let compile_unit env = compile_unboxed_const (unit_vanilla_lit ())

  (* Expects on the stack the pointer to the array. *)
  let load_n env n =
    Tagged.load_forwarding_pointer env ^^
    Tagged.(sanity_check_tag __LINE__ env (Array T)) ^^
    Tagged.load_field env (Int64.add Arr.header_size n)

  (* Takes n elements of the stack and produces an argument tuple *)
  let from_stack env n =
    if n = 0 then compile_unit env
    else
      let name = Printf.sprintf "to_%i_tuple" n in
      let args = Lib.List.table n (fun i -> Printf.sprintf "arg%i" i, I64Type) in
      Func.share_code Func.Never env name args [I64Type] (fun env getters ->
        Arr.lit env Tagged.T (Lib.List.table n (fun i -> List.nth getters i))
      )

  (* Takes an argument tuple and puts the elements on the stack: *)
  let to_stack env n =
    if n = 0 then G.i Drop else
    begin
      let name = Printf.sprintf "from_%i_tuple" n in
      let retty = Lib.List.make n I64Type in
      Func.share_code1 Func.Never env name ("tup", I64Type) retty (fun env get_tup ->
        G.table n (fun i -> get_tup ^^ load_n env (Int64.of_int i))
      )
    end

end (* Tuple *)

module Lifecycle = struct
  (*
  This module models the life cycle of a canister as a very simple state machine,
  keeps track of the current state of the canister, and traps noisily if an
  unexpected transition happens. Such a transition would either be a bug in the
  underlying system, or in our RTS.
  *)

  type state =
    | PreInit
  (* We do not use the (start) function when compiling canisters, so skip
     these two:
    | InStart
    | Started (* (start) has run *)
  *)
    | InInit (* canister_init *)
    | Idle (* basic steady state *)
    | InUpdate
    | InQuery
    | PostQuery (* an invalid state *)
    | InPreUpgrade
    | PostPreUpgrade (* an invalid state *)
    | InPostUpgrade
    | InComposite
    | InStabilization (* stabilization before upgrade *)
    | InDestabilization (* destabilization after upgrade *)

  let string_of_state state = match state with
    | PreInit -> "PreInit"
    | InInit -> "InInit"
    | Idle -> "Idle"
    | InUpdate -> "InUpdate"
    | InQuery -> "InQuery"
    | PostQuery -> "PostQuery"
    | InPreUpgrade -> "InPreUpgrade"
    | PostPreUpgrade -> "PostPreUpgrade"
    | InPostUpgrade -> "InPostUpgrade"
    | InComposite -> "InComposite"
    | InStabilization -> "InStabilization"
    | InDestabilization -> "InDestabilization"

  let int_of_state = function
    | PreInit -> 0L (* Automatically null *)
    (*
    | InStart -> 1L
    | Started -> 2L
    *)
    | InInit -> 3L
    | Idle -> 4L
    | InUpdate -> 5L
    | InQuery -> 6L
    | PostQuery -> 7L
    | InPreUpgrade -> 8L
    | PostPreUpgrade -> 9L
    | InPostUpgrade -> 10L
    | InComposite -> 11L
    | InStabilization -> 12L
    | InDestabilization -> 13L

  let ptr () = Stack.end_ ()
  let end_ () = Int64.add (Stack.end_ ()) Heap.word_size

  (* Which states may come before this *)
  let pre_states = function
    | PreInit -> []
    (*
    | InStart -> [PreInit]
    | Started -> [InStart]
    *)
    | InInit -> [PreInit]
    | Idle -> [InInit; InUpdate; InPostUpgrade; InComposite; InDestabilization]
    | InUpdate -> [Idle]
    | InQuery -> [Idle]
    | PostQuery -> [InQuery]
    | InPreUpgrade -> [Idle; InStabilization]
    | PostPreUpgrade -> [InPreUpgrade]
    | InPostUpgrade -> [InInit; InDestabilization]
    | InComposite -> [Idle; InComposite]
    | InStabilization -> [Idle; InStabilization]
    | InDestabilization -> [InInit]

  let get env =
    compile_unboxed_const (ptr ()) ^^
    load_unskewed_ptr

  let set env new_state =
    compile_unboxed_const (ptr ()) ^^
    compile_unboxed_const (int_of_state new_state) ^^
    store_unskewed_ptr

  let during_explicit_upgrade env =
    get env ^^
    compile_eq_const (int_of_state InStabilization) ^^
    get env ^^
    compile_eq_const (int_of_state InDestabilization) ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or))

  let trans env new_state =
    let name = "trans_state" ^ Int64.to_string (int_of_state new_state) in
    Func.share_code0 Func.Always env name [] (fun env ->
      G.block0 (
        let rec go = function
        | [] ->
          during_explicit_upgrade env ^^
          E.if0
            (E.trap_with env "Messages are blocked during stabilization")
            (E.trap_with env
              ("internal error: unexpected state entering " ^ string_of_state new_state))
        | (s::ss) ->
          get env ^^ compile_eq_const (int_of_state s) ^^
          E.if0 (G.i (Br (nr 1l))) G.nop ^^
          go ss
        in go (pre_states new_state)
        ) ^^
      set env new_state
    )

  let is_in env state =
    get env ^^
    compile_eq_const (int_of_state state)

end (* Lifecycle *)


module IC = struct

  (* IC-specific stuff: System imports, databufs etc. *)

  (* Stands for the `I` value from the 'IC Interface Specification'.
   * Use it where a pointer type is expected, to easily differentiate between pointers and i32/i64 values.
   *)
  let i = I64Type

  let register_globals env =
    (* result of last ic0.call_perform  *)
    E.add_global64 env "__call_perform_status" Mutable 0L;
    E.add_global64 env "__call_perform_message" Mutable 0L;
    E.add_global64 env "__run_post_upgrade" Mutable 0L
    (* NB: __call_perform_message is not a root so text contents *must* be static *)

  let get_call_perform_status env =
    G.i (GlobalGet (nr (E.get_global env "__call_perform_status")))
  let set_call_perform_status env =
    G.i (GlobalSet (nr (E.get_global env "__call_perform_status")))
  let get_call_perform_message env =
    G.i (GlobalGet (nr (E.get_global env "__call_perform_message")))
  let set_call_perform_message env =
    G.i (GlobalSet (nr (E.get_global env "__call_perform_message")))
  let get_run_post_upgrade env =
    G.i (GlobalGet (nr (E.get_global env "__run_post_upgrade")))
  let set_run_post_upgrade env =
    G.i (GlobalSet (nr (E.get_global env "__run_post_upgrade")))

  let init_globals env =
    Blob.lit env Tagged.T "" ^^
    set_call_perform_message env

  let is n = Lib.List.make n i
  let i64s n = Lib.List.make n I64Type

  let import_ic0 env =
    (* Keep all the imports in sync between classical and enhanced versions *)
    E.add_func_import env "ic0" "accept_message" [] [];
    E.add_func_import env "ic0" "call_data_append" (is 2) [];
    E.add_func_import env "ic0" "call_cycles_add128" (i64s 2) [];
    E.add_func_import env "ic0" "call_with_best_effort_response" [I32Type] [];
    E.add_func_import env "ic0" "call_new" (is 8) [];
    E.add_func_import env "ic0" "call_perform" [] [I32Type];
    E.add_func_import env "ic0" "call_on_cleanup" (is 2) [];
    E.add_func_import env "ic0" "canister_cycle_balance128" [i] [];
    E.add_func_import env "ic0" "canister_self_copy" (is 3) [];
    E.add_func_import env "ic0" "canister_self_size" [] [i];
    E.add_func_import env "ic0" "canister_status" [] [I32Type];
    E.add_func_import env "ic0" "canister_version" [] [I64Type];
    E.add_func_import env "ic0" "root_key_copy" (is 3) [];
    E.add_func_import env "ic0" "root_key_size" [] [i];
    E.add_func_import env "ic0" "in_replicated_execution" [] [I32Type];
    E.add_func_import env "ic0" "is_controller" (is 2) [I32Type];
    E.add_func_import env "ic0" "subnet_self_copy" (is 3) [];
    E.add_func_import env "ic0" "subnet_self_size" [] [i];
    E.add_func_import env "ic0" "debug_print" (is 2) [];
    E.add_func_import env "ic0" "msg_arg_data_copy" (is 3) [];
    E.add_func_import env "ic0" "msg_arg_data_size" [] [i];
    E.add_func_import env "ic0" "msg_caller_copy" (is 3) [];
    E.add_func_import env "ic0" "msg_caller_size" [] [i];
    E.add_func_import env "ic0" "msg_cycles_available128" [i] [];
    E.add_func_import env "ic0" "msg_cycles_refunded128" [i] [];
    E.add_func_import env "ic0" "msg_cycles_accept128" [I64Type; I64Type; i] [];
    E.add_func_import env "ic0" "cycles_burn128" [I64Type; I64Type; i] [];

    (* Cost *)
    E.add_func_import env "ic0" "cost_call" [I64Type; I64Type; i] [];
    E.add_func_import env "ic0" "cost_create_canister" [i] [];
    E.add_func_import env "ic0" "cost_http_request" [I64Type; I64Type; i] [];
    E.add_func_import env "ic0" "cost_sign_with_ecdsa" [i; i; I32Type; i] [I32Type];
    E.add_func_import env "ic0" "cost_sign_with_schnorr" [i; i; I32Type; i] [I32Type];

    E.add_func_import env "ic0" "certified_data_set" (is 2) [];
    E.add_func_import env "ic0" "data_certificate_present" [] [I32Type];
    E.add_func_import env "ic0" "data_certificate_size" [] [i];
    E.add_func_import env "ic0" "data_certificate_copy" (is 3) [];
    E.add_func_import env "ic0" "msg_method_name_size" [] [i];
    E.add_func_import env "ic0" "msg_method_name_copy" (is 3) [];
    E.add_func_import env "ic0" "msg_reject_code" [] [I32Type];
    E.add_func_import env "ic0" "msg_reject_msg_size" [] [i];
    E.add_func_import env "ic0" "msg_reject_msg_copy" (is 3) [];
    E.add_func_import env "ic0" "msg_reject" (is 2) [];
    E.add_func_import env "ic0" "msg_reply_data_append" (is 2) [];
    E.add_func_import env "ic0" "msg_reply" [] [];
    E.add_func_import env "ic0" "msg_deadline" [] [I64Type];
    E.add_func_import env "ic0" "performance_counter" [I32Type] [I64Type];
    E.add_func_import env "ic0" "trap" (is 2) [];
    E.add_func_import env "ic0" "stable64_write" (i64s 3) [];
    E.add_func_import env "ic0" "stable64_read" (i64s 3) [];
    E.add_func_import env "ic0" "stable64_size" [] [I64Type];
    E.add_func_import env "ic0" "stable64_grow" [I64Type] [I64Type];
    E.add_func_import env "ic0" "env_var_count" [] [i];
    E.add_func_import env "ic0" "env_var_name_size" [i] [i];
    E.add_func_import env "ic0" "env_var_name_copy" (is 4) [];
    E.add_func_import env "ic0" "env_var_name_exists" [i; i] [I32Type];
    E.add_func_import env "ic0" "env_var_value_size" [i; i] [i];
    E.add_func_import env "ic0" "env_var_value_copy" (is 5) [];
    E.add_func_import env "ic0" "time" [] [I64Type];
    if !Flags.global_timer then
      E.add_func_import env "ic0" "global_timer_set" [I64Type] [I64Type]

  let system_imports env =
    match E.mode env with
    | Flags.ICMode ->
      import_ic0 env
    | Flags.RefMode  ->
      import_ic0 env
    | Flags.WASIMode ->
      (* Wasi function is still 32-bit based *)
      E.add_func_import env "wasi_snapshot_preview1" "fd_write" [I32Type; I32Type; I32Type; I32Type] [I32Type];
    | Flags.WasmMode -> ()

  let system_call env funcname = E.call_import env "ic0" funcname

  let register env =
      let min env first second =
        first ^^
        second ^^
        compile_comparison I64Op.LtU ^^
        E.if1 I64Type (first) (second) in

      Func.define_built_in env "print_ptr" [("ptr", I64Type); ("len", I64Type)] [] (fun env ->
        match E.mode env with
        | Flags.WasmMode -> G.i Nop
        | Flags.ICMode | Flags.RefMode ->
          G.i (LocalGet (nr 0l)) ^^
          G.i (LocalGet (nr 1l)) ^^
          system_call env "debug_print"
        | Flags.WASIMode -> begin
          (* Since the wasmtime `fd_write` function still only supports 32-bit pointers in 64-bit mode,
             we use a static buffer for the text output that resides in the 32-bit space.
             This buffer is reserved is limited to 512 bytes and is managed in the RTS, see `buffer_in_32_bit_range()`. *)
          let get_ptr = G.i (LocalGet (nr 0l)) in
          let get_len = G.i (LocalGet (nr 1l)) in

          Stack.with_words env "io_vec" 6L (fun get_iovec_ptr ->
            let buffer_length = 512 in
            let buffer_ptr = E.call_import env "rts" "buffer_in_32_bit_range" in

            (* Truncate the text if it does not fit into the buffer **)
            min env (compile_unboxed_const (Int64.of_int buffer_length)) get_len ^^
            G.setter_for get_len ^^

            (* Copy the text to the static buffer in 32-bit space *)
            buffer_ptr ^^
            get_ptr ^^
            get_len ^^
            Heap.memcpy env ^^

            (* We use the iovec functionality to append a newline *)
            get_iovec_ptr ^^
            narrow_to_32 env buffer_ptr ^^ (* This is safe because the buffer resides in 32-bit space *)
            G.i (Store {ty = I32Type; align = 2; offset = 0L; sz = None}) ^^

            get_iovec_ptr ^^
            narrow_to_32 env get_len ^^
            G.i (Store {ty = I32Type; align = 2; offset = 4L; sz = None}) ^^

            get_iovec_ptr ^^
            narrow_to_32 env get_iovec_ptr ^^ (* The stack pointer should always be in the 32-bit space *)
            compile_add32_const 16l ^^
            G.i (Store {ty = I32Type; align = 2; offset = 8L; sz = None}) ^^

            get_iovec_ptr ^^
            compile_const_32 1l ^^
            G.i (Store {ty = I32Type; align = 2; offset = 12L; sz = None}) ^^

            get_iovec_ptr ^^
            compile_const_32 (Int32.of_int (Char.code '\n')) ^^
            G.i (Store {ty = I32Type; align = 0; offset = 16L; sz = Some Wasm_exts.Types.Pack8}) ^^

            (* Call fd_write twice to work around
               https://github.com/bytecodealliance/wasmtime/issues/629
            *)

            compile_const_32 1l (* stdout *) ^^
            narrow_to_32 env get_iovec_ptr ^^
            compile_const_32 1l (* one string segment (2 doesn't work) *) ^^
            narrow_to_32 env get_iovec_ptr ^^
            compile_add32_const 20l ^^ (* out for bytes written, we ignore that *)
            E.call_import env "wasi_snapshot_preview1" "fd_write" ^^
            G.i Drop ^^

            compile_const_32 1l (* stdout *) ^^
            narrow_to_32 env get_iovec_ptr ^^
            compile_add32_const 8l ^^
            compile_const_32 1l (* one string segment *) ^^
            narrow_to_32 env get_iovec_ptr ^^
            compile_add32_const 20l ^^ (* out for bytes written, we ignore that *)
            E.call_import env "wasi_snapshot_preview1" "fd_write" ^^
            G.i Drop)
          end);

      E.add_export env (nr {
        name = Lib.Utf8.decode "print_ptr";
        edesc = nr (FuncExport (nr (E.built_in env "print_ptr")))
      })


  let ic_system_call call env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env call
    | _ ->
      E.trap_with env Printf.(sprintf "cannot get %s when running locally" call)

  let performance_counter env =
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    ic_system_call "performance_counter" env

  let is_controller env =
    ic_system_call "is_controller" env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))

  let replicated_execution env =
    ic_system_call "in_replicated_execution" env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))

  let canister_version env = ic_system_call "canister_version" env

  let print_ptr_len env = G.i (Call (nr (E.built_in env "print_ptr")))

  let print_text env =
    Func.share_code1 Func.Never env "print_text" ("str", I64Type) [] (fun env get_str ->
      let (set_blob, get_blob) = new_local env "blob" in
      get_str ^^ Text.to_blob env ^^ set_blob ^^
      get_blob ^^ Blob.payload_ptr_unskewed env ^^
      get_blob ^^ Blob.len env ^^
      print_ptr_len env
    )

  (* For debugging *)
  let _compile_static_print env s =
    Blob.lit_ptr_len env Tagged.T s ^^ print_ptr_len env

  let ic_trap env =
    Func.share_code2 Func.Always env "ic_trap" (("ptr", I64Type), ("len", I64Type)) [] (fun env get_ptr get_length ->
      get_ptr ^^
      get_length ^^
      system_call env "trap"
    )

  let trap_ptr_len env =
    match E.mode env with
    | Flags.WasmMode -> G.i Unreachable
    | Flags.WASIMode -> print_ptr_len env ^^ G.i Unreachable
    | Flags.ICMode | Flags.RefMode -> ic_trap env ^^ G.i Unreachable

  let trap_with env s =
    (* TODO: instead of pre-allocating a shared constant, allocate
       s from a segment to reduce pool size *)
    Blob.lit_ptr_len env Tagged.T s ^^ trap_ptr_len env

  let trap_text env  =
    Text.to_blob env ^^ Blob.as_ptr_len env ^^ trap_ptr_len env

  let default_exports env =
    (* these exports seem to be wanted by the hypervisor/v8 *)
    E.add_export env (nr {
      name = Lib.Utf8.decode (
        match E.mode env with
        | Flags.WASIMode -> "memory"
        | _  -> "mem"
      );
      edesc = nr (MemoryExport (nr 0l))
    });
    E.add_export env (nr {
      name = Lib.Utf8.decode "table";
      edesc = nr (TableExport (nr 0l))
    })

  let export_init env =
    assert (E.mode env = Flags.ICMode || E.mode env = Flags.RefMode);
    let empty_f = Func.of_body env [] [] (fun env ->
      Lifecycle.trans env Lifecycle.InInit ^^
      G.i (Call (nr (E.built_in env "init")))
      (* Stay in `InInit` state for asynchronous destabilization after upgrade. *)
      (* Garbage collection is not yet activated. *)

    ) in
    let fi = E.add_fun env "canister_init" empty_f in
    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_init";
      edesc = nr (FuncExport (nr fi))
      })

  let export_heartbeat env =
    assert (E.mode env = Flags.ICMode || E.mode env = Flags.RefMode);
    let fi = E.add_fun env "canister_heartbeat"
      (Func.of_body env [] [] (fun env ->
        G.i (Call (nr (E.built_in env "heartbeat_exp"))) ^^
        GC.collect_garbage env))
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_heartbeat";
      edesc = nr (FuncExport (nr fi))
    })

  let export_timer env =
    assert !Flags.global_timer;
    assert (E.mode env = Flags.ICMode || E.mode env = Flags.RefMode);
    let fi = E.add_fun env "canister_global_timer"
      (Func.of_body env [] [] (fun env ->
        G.i (Call (nr (E.built_in env "timer_exp"))) ^^
        GC.collect_garbage env))
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_global_timer";
      edesc = nr (FuncExport (nr fi))
    })

  let export_inspect env =
    assert (E.mode env = Flags.ICMode || E.mode env = Flags.RefMode);
    let fi = E.add_fun env "canister_inspect_message"
      (Func.of_body env [] [] (fun env ->
        G.i (Call (nr (E.built_in env "inspect_exp"))) ^^
        system_call env "accept_message" (* assumes inspect_exp traps to reject *)
        (* no need to GC !*)))
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_inspect_message";
      edesc = nr (FuncExport (nr fi))
    })

  let export_low_memory env =
    assert (E.mode env = Flags.ICMode || E.mode env = Flags.RefMode);
    let fi = E.add_fun env "canister_on_low_wasm_memory"
      (Func.of_body env [] [] (fun env ->
        G.i (Call (nr (E.built_in env "low_memory_exp"))) ^^
        GC.collect_garbage env))
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_on_low_wasm_memory";
      edesc = nr (FuncExport (nr fi))
    })

  let initialize_main_actor_function_name = "@initialize_main_actor"

  let initialize_main_actor env =
    G.i (Call (nr (E.built_in env initialize_main_actor_function_name))) ^^
    get_run_post_upgrade env ^^
    (E.if0
      begin
        Lifecycle.trans env Lifecycle.InPostUpgrade ^^
        G.i (Call (nr (E.built_in env "post_exp")))
      end
      G.nop)

  let get_actor_to_persist_function_name = "@get_actor_to_persist"

  let get_actor_to_persist env =
    G.i (Call (nr (E.built_in env get_actor_to_persist_function_name)))

  let export_wasi_start env =
    assert (E.mode env = Flags.WASIMode);
    let fi = E.add_fun env "_start" (Func.of_body env [] [] (fun env1 ->
      Lifecycle.trans env Lifecycle.InInit ^^
      G.i (Call (nr (E.built_in env "init"))) ^^
      (if !Flags.sanity then
        (* also test the GC in WASI mode if sanity checks are enabled *)
        GC.collect_garbage env
      else
        G.nop) ^^
      Lifecycle.trans env Lifecycle.Idle
    )) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "_start";
      edesc = nr (FuncExport (nr fi))
      })

  let export_upgrade_methods env =
    if E.mode env = Flags.ICMode || E.mode env = Flags.RefMode then
    let status_stopped = 3l in
    let pre_upgrade_fi = E.add_fun env "pre_upgrade" (Func.of_body env [] [] (fun env ->
      Lifecycle.trans env Lifecycle.InPreUpgrade ^^
      (* check status is stopped or trap on outstanding callbacks *)
      system_call env "canister_status" ^^ compile_eq32_const status_stopped ^^
      E.if0
       (G.nop)
       (ContinuationTable.count env ^^
          E.then_trap_with env "canister_pre_upgrade attempted with outstanding message callbacks (try stopping the canister before upgrade)") ^^
      (* call pre_upgrade expression & any system method *)
      (G.i (Call (nr (E.built_in env "pre_exp")))) ^^
      Lifecycle.trans env Lifecycle.PostPreUpgrade
    )) in

    let post_upgrade_fi = E.add_fun env "post_upgrade" (Func.of_body env [] [] (fun env ->
      compile_unboxed_one ^^ set_run_post_upgrade env ^^
      Lifecycle.trans env Lifecycle.InInit ^^
      G.i (Call (nr (E.built_in env "init")))
      (* The post upgrade hook is called later after the completed destabilization,
         that may require additional explicit destabilization messages after upgrade. *)
    )) in

    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_pre_upgrade";
      edesc = nr (FuncExport (nr pre_upgrade_fi))
    });

    E.add_export env (nr {
      name = Lib.Utf8.decode "canister_post_upgrade";
      edesc = nr (FuncExport (nr post_upgrade_fi))
    })


  let get_self_reference env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.share_code0 Func.Never env "canister_self" [I64Type] (fun env ->
        Blob.of_size_copy env Tagged.A
          (fun env ->
            system_call env "canister_self_size")
          (fun env ->
            system_call env "canister_self_copy")
          (fun env -> compile_unboxed_const 0L)
      )
    | _ ->
      E.trap_with env "cannot get self-actor-reference when running locally"

  let get_subnet_reference env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      Func.share_code0 Func.Never env "canister_subnet" [I64Type] (fun env ->
        Blob.of_size_copy env Tagged.A
          (fun env -> system_call env "subnet_self_size")
          (fun env -> system_call env "subnet_self_copy")
          (fun env -> compile_unboxed_const 0L)
      )
    | _ ->
      E.trap_with env "cannot get actor-subnet-reference when running locally"

  let get_root_key env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      Func.share_code0 Func.Never env "root_key" [i] (fun env ->
        Blob.of_size_copy env Tagged.A
          (fun env -> system_call env "root_key_size")
          (fun env -> system_call env "root_key_copy")
          (fun env -> compile_unboxed_const 0L)
      )
    | _ ->
      E.trap_with env "cannot get root-key when running locally"

  let get_system_time env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      system_call env "time"
    | _ ->
      E.trap_with env "cannot get system time when running locally"

  let env_var_names env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      Func.share_code0 Func.Never env "env_var_names" [i] (fun env ->
        let (set_len, get_len) = new_local env "len" in
        let (set_array, get_array) = new_local env "array" in
        system_call env "env_var_count" ^^ set_len ^^
        Arr.alloc env Tagged.I get_len ^^ set_array ^^
        get_len ^^ from_0_to_n env (fun get_i ->
          let (set_name_len, get_name_len) = new_local env "name_len" in
          let (set_name, get_name) = new_local env "name" in

          get_i ^^
          system_call env "env_var_name_size" ^^ set_name_len ^^

          Blob.alloc env Tagged.T get_name_len ^^ set_name ^^

          get_array ^^ get_i ^^ Arr.unsafe_idx env ^^
          get_name ^^
          store_ptr ^^

          get_i ^^
          get_name ^^ Blob.payload_ptr_unskewed env ^^
          compile_unboxed_zero ^^
          get_name_len ^^
          system_call env "env_var_name_copy" ^^

          get_name
        ) ^^
        get_array ^^
        Tagged.allocation_barrier env
      )
    | _ ->
      E.trap_with env "cannot get environment variable names when running locally"

  let env_var env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      Func.share_code1 Func.Never env "env_var" ("name", i) [i] (fun env get_name ->
        let (set_name_len, get_name_len) = new_local env "name_len" in

        get_name ^^ Blob.len env ^^ set_name_len ^^

        get_name ^^ Blob.payload_ptr_unskewed env ^^
        get_name_len ^^
        system_call env "env_var_name_exists" ^^
        Bool.from_rts_int32 ^^
        E.if1 I64Type (
          let (set_value_len, get_value_len) = new_local env "value_len" in
          let (set_value, get_value) = new_local env "value" in

          get_name ^^ Blob.payload_ptr_unskewed env ^^
          get_name_len ^^
          system_call env "env_var_value_size" ^^ set_value_len ^^

          Blob.alloc env Tagged.T get_value_len ^^ set_value ^^

          get_name ^^ Blob.payload_ptr_unskewed env ^^
          get_name_len ^^
          get_value ^^ Blob.payload_ptr_unskewed env ^^
          compile_unboxed_zero ^^
          get_value_len ^^
          system_call env "env_var_value_copy" ^^

          Opt.inject_simple env get_value)
        (Opt.null_lit env)
      )
    | _ ->
      E.trap_with env "cannot get environment variable when running locally"

  let caller env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Blob.of_size_copy env Tagged.P
        (fun env ->
          system_call env "msg_caller_size")
        (fun env ->
          system_call env "msg_caller_copy")
        (fun env -> compile_unboxed_const 0L)
    | _ ->
      E.trap_with env "cannot get caller when running locally"

  let method_name env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Blob.of_size_copy env Tagged.T
        (fun env ->
          system_call env "msg_method_name_size")
        (fun env ->
          system_call env "msg_method_name_copy")
        (fun env -> compile_unboxed_const 0L)
    | _ ->
      E.trap_with env "cannot get method_name when running locally"

  let arg_data env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Blob.of_size_copy env Tagged.B
        (fun env ->
          system_call env "msg_arg_data_size")
        (fun env ->
          system_call env "msg_arg_data_copy")
        (fun env -> compile_unboxed_const 0L)
    | _ ->
      E.trap_with env "cannot get arg_data when running locally"

  let deadline env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "msg_deadline"
    | _ ->
      E.trap_with env "cannot get deadline when running locally"

  let reject env arg_instrs =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      arg_instrs ^^
      Text.to_blob env ^^
      Blob.as_ptr_len env ^^
      system_call env "msg_reject"
    | _ ->
      E.trap_with env "cannot reject when running locally"

  let error_code env =
     Func.share_code0 Func.Always env "error_code" [I64Type] (fun env ->
      let (set_code, get_code) = new_local env "code" in
      system_call env "msg_reject_code" ^^
      G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
      set_code ^^
      List.fold_right (fun (tag, const) code ->
        get_code ^^ compile_unboxed_const const ^^
        compile_comparison I64Op.Eq ^^
        E.if1 I64Type
          (Variant.inject env tag (Tuple.compile_unit env))
          code)
        ["system_fatal", 1L;
         "system_transient", 2L;
         "destination_invalid", 3L;
         "canister_reject", 4L;
         "canister_error", 5L;
         "system_unknown", 6L]
        (Variant.inject env "future" (get_code ^^ BitTagged.tag env Type.Nat32)))

  let error_message env =
    Func.share_code0 Func.Never env "error_message" [I64Type] (fun env ->
      Blob.of_size_copy env Tagged.T
        (fun env -> system_call env "msg_reject_msg_size")
        (fun env ->
          system_call env "msg_reject_msg_copy")
        (fun env -> compile_unboxed_const 0L)
    )

  let error_value env =
    Func.share_code0 Func.Never env "error_value" [I64Type] (fun env ->
      error_code env ^^
      error_message env ^^
      Tuple.from_stack env 2
    )

  let reply_with_data env =
    Func.share_code2 Func.Never env "reply_with_data" (("start", I64Type), ("size", I64Type)) [] (
      fun env get_data_start get_data_size ->
        get_data_start ^^
        get_data_size ^^
        system_call env "msg_reply_data_append" ^^
        system_call env "msg_reply"
   )

  let static_nullary_reply env =
    Blob.lit_ptr_len env Tagged.B "DIDL\x00\x00" ^^
    reply_with_data env

  (* Actor reference on the stack *)
  let actor_public_field env name =
    (* simply tuple canister name and function name *)
    Tagged.(sanity_check_tag __LINE__ env (Blob A)) ^^
    Blob.lit env Tagged.T name ^^
    Func.share_code2 Func.Never env "actor_public_field" (("actor", I64Type), ("func", I64Type)) [I64Type] (
      fun env get_actor get_func ->
      Arr.lit env Tagged.S [get_actor; get_func]
   )


  let fail_assert env at =
    let open Source in
    let at = {
        left = {at.left with file = Filename.basename at.left.file};
        right = {at.right with file = Filename.basename at.right.file}
      }
    in
    E.trap_with env (Printf.sprintf "assertion failed at %s" (string_of_region at))

  let async_method_name = Type.(motoko_async_helper_fld.lab)
  let gc_trigger_method_name = Type.(motoko_gc_trigger_fld.lab)

  let is_self_call env =
    let (set_len_self, get_len_self) = new_local env "len_self" in
    let (set_len_caller, get_len_caller) = new_local env "len_caller" in
    system_call env "canister_self_size" ^^ set_len_self ^^
    system_call env "msg_caller_size" ^^ set_len_caller ^^
    get_len_self ^^ get_len_caller ^^ compile_comparison I64Op.Eq ^^
    E.if1 I64Type
      begin
        get_len_self ^^ Stack.dynamic_with_bytes env "str_self" (fun get_str_self ->
          get_len_caller ^^ Stack.dynamic_with_bytes env "str_caller" (fun get_str_caller ->
            get_str_caller ^^ compile_unboxed_const 0L ^^ get_len_caller ^^
            system_call env "msg_caller_copy" ^^
            get_str_self ^^ compile_unboxed_const 0L ^^ get_len_self ^^
            system_call env "canister_self_copy" ^^
            get_str_self ^^ get_str_caller ^^ get_len_self ^^ Heap.memcmp env ^^
            compile_eq_const 0L))
      end
      begin
        compile_unboxed_const 0L
      end

  let assert_caller_self env =
    is_self_call env ^^
    E.else_trap_with env "not a self-call"

  let is_controller_call env =
    let (set_len_caller, get_len_caller) = new_local env "len_caller" in
    system_call env "msg_caller_size" ^^ set_len_caller ^^
    get_len_caller ^^ Stack.dynamic_with_bytes env "str_caller" (fun get_str_caller ->
      get_str_caller ^^ compile_unboxed_const 0L ^^ get_len_caller ^^
      system_call env "msg_caller_copy" ^^
      get_str_caller ^^ get_len_caller ^^ is_controller env)

  let assert_caller_self_or_controller env =
    is_self_call env ^^
    is_controller_call env ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
    E.else_trap_with env "not a self-call or call from controller"

  (* Cycles *)

  let cycle_balance env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "canister_cycle_balance128"
    | _ ->
      E.trap_with env "cannot read balance when running locally"

  let cycles_add env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "call_cycles_add128"
    | _ ->
      E.trap_with env "cannot accept cycles when running locally"

  let cycles_accept env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "msg_cycles_accept128"
    | _ ->
      E.trap_with env "cannot accept cycles when running locally"

  let cycles_available env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "msg_cycles_available128"
    | _ ->
      E.trap_with env "cannot get cycles available when running locally"

  let cycles_refunded env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "msg_cycles_refunded128"
    | _ ->
      E.trap_with env "cannot get cycles refunded when running locally"

  let cycles_burn env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "cycles_burn128"
    | _ ->
      E.trap_with env "cannot burn cycles when running locally"

  let set_certified_data env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      Blob.as_ptr_len env ^^
      system_call env "certified_data_set"
    | _ ->
      E.trap_with env "cannot set certified data when running locally"

  let get_certificate env =
    match E.mode env with
    | Flags.(ICMode | RefMode) ->
      system_call env "data_certificate_present" ^^
      Bool.from_rts_int32 ^^
      E.if1 I64Type
      begin
        Opt.inject_simple env (
          Blob.of_size_copy env Tagged.B
            (fun env ->
              system_call env "data_certificate_size")
            (fun env ->
              system_call env "data_certificate_copy")
            (fun env -> compile_unboxed_const 0L)
        )
      end (Opt.null_lit env)
    | _ ->
      E.trap_with env "cannot get certificate when running locally"

end (* IC *)

module Cycles = struct

  let from_word128_ptr env = Func.share_code1 Func.Never env "from_word128_ptr" ("ptr", I64Type) [I64Type]
    (fun env get_ptr ->
     let set_lower, get_lower = new_local env "lower" in
     get_ptr ^^
     G.i (Load {ty = I64Type; align = 0; offset = 0L; sz = None }) ^^
     BigNum.from_word64 env ^^
     set_lower ^^
     get_ptr ^^
     G.i (Load {ty = I64Type; align = 0; offset = 8L; sz = None }) ^^
     compile_test I64Op.Eqz ^^
     E.if1 I64Type
       get_lower
       begin
         get_lower ^^
         get_ptr ^^
         G.i (Load {ty = I64Type; align = 0; offset = 8L; sz = None }) ^^
         BigNum.from_word64 env ^^
         (* shift left 64 bits *)
         compile_unboxed_const 64L ^^
         TaggedSmallWord.msb_adjust Type.Nat32 ^^
         BigNum.compile_lsh env ^^
         BigNum.compile_add env
       end)

  (* takes a bignum from the stack, traps if ≥2^128, and leaves two 64bit words on the stack *)
  (* only used twice, so ok to not use share_code1; that would require I64Type support in FakeMultiVal *)
  let to_two_word64 env =
    let (set_val, get_val) = new_local env "cycles" in
    set_val ^^
    get_val ^^
    Tagged.materialize_shared_value env (BigNum.constant env (Big_int.power_int_positive_int 2 128)) ^^
    BigNum.compile_relop env Lt ^^
    E.else_trap_with env "cycles out of bounds" ^^

    get_val ^^
    (* shift right 64 bits *)
    compile_unboxed_const 64L ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    BigNum.compile_rsh env ^^
    BigNum.truncate_to_word64 env ^^

    get_val ^^
    BigNum.truncate_to_word64 env

  let balance env =
    Func.share_code0 Func.Always env "cycle_balance" [I64Type] (fun env ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_dst ^^
        IC.cycle_balance env ^^
        get_dst ^^
        from_word128_ptr env
      )
    )

  let add env =
    Func.share_code1 Func.Always env "cycle_add" ("cycles", I64Type) [] (fun env get_x ->
      get_x ^^
      to_two_word64 env ^^
      IC.cycles_add env
    )

  let accept env =
    Func.share_code1 Func.Always env "cycle_accept" ("cycles", I64Type) [I64Type] (fun env get_x ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_x ^^
        to_two_word64 env ^^
        get_dst ^^
        IC.cycles_accept env ^^
        get_dst ^^
        from_word128_ptr env
      )
    )

  let available env =
    Func.share_code0 Func.Always env "cycle_available" [I64Type] (fun env ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_dst ^^
        IC.cycles_available env ^^
        get_dst ^^
        from_word128_ptr env
      )
    )

  let refunded env =
    Func.share_code0 Func.Always env "cycle_refunded" [I64Type] (fun env ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_dst ^^
        IC.cycles_refunded env ^^
        get_dst ^^
        from_word128_ptr env
      )
    )

  let burn env =
    Func.share_code1 Func.Always env "cycle_burn" ("cycles", I64Type) [I64Type] (fun env get_x ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_x ^^
        to_two_word64 env ^^
        get_dst ^^
        IC.cycles_burn env ^^
        get_dst ^^
        from_word128_ptr env
      )
    )

end (* Cycles *)

(* Low-level, almost raw access to IC stable memory.
   Essentially a virtual page allocator
   * enforcing limit --max-stable-pages not exceeded
   * tracking virtual page count, ignoring physical pages added for stable variable serialization (global`__stable_mem_size`)
   * recording current format of contents (global `__stable_version`)
   Used to implement stable variable serialization, (experimental) stable memory library and Region type (see region.rs)
*)
module StableMem = struct


  (* Raw stable memory API,
     using ic0.stable64_xxx or
     emulating via (for now) 64-bit memory 1
  *)
  let stable64_grow env =
    E.require_stable_memory env;
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
       IC.system_call env "stable64_grow"
    | _ ->
       Func.share_code1 Func.Always env "stable64_grow" ("pages", I64Type) [I64Type]
         (fun env get_pages ->
          let set_old_pages, get_old_pages = new_local env "old_pages" in
          get_pages ^^
          G.i StableGrow ^^
          set_old_pages ^^
          get_old_pages ^^
          compile_unboxed_const (-1L) ^^
          compile_comparison I64Op.Eq ^^
          E.if1 I64Type
            begin
             compile_unboxed_const (-1L)
            end
            begin
              get_old_pages
            end)

  let stable64_size env =
    E.require_stable_memory env;
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
       IC.system_call env "stable64_size"
    | _ ->
       Func.share_code0 Func.Always env "stable64_size" [I64Type]
         (fun env ->
          G.i StableSize)

  let stable64_read env =
    E.require_stable_memory env;
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
       IC.system_call env "stable64_read"
    | _ ->
       Func.share_code3 Func.Always env "stable64_read"
         (("dst", I64Type), ("offset", I64Type), ("size", I64Type)) []
         (fun env get_dst get_offset get_size ->
          get_dst ^^
          get_offset ^^
          get_size ^^
          G.i StableRead)

  let stable64_write env =
    E.require_stable_memory env;
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
       IC.system_call env "stable64_write"
    | _ ->
       Func.share_code3 Func.Always env "stable64_write"
         (("offset", I64Type), ("src", I64Type), ("size", I64Type)) []
         (fun env get_offset get_src get_size ->
          get_offset ^^
          get_src ^^
          get_size ^^
          G.i StableWrite)


  (* Versioning (c.f. Region.rs) *)
  (* NB: these constants must agree with the constants in Region.rs *)
  let legacy_version_no_stable_memory = Int64.of_int 0 (* never manifest in serialized form *)
  let legacy_version_some_stable_memory = Int64.of_int 1
  let legacy_version_regions = Int64.of_int 2
  let version_graph_copy_no_regions = Int64.of_int 3
  let version_graph_copy_regions = Int64.of_int 4
  let version_stable_heap_no_regions = Int64.of_int 5
  let version_stable_heap_regions = Int64.of_int 6
  let version_max = version_stable_heap_regions

  let register_globals env =
    (* size (in pages) *)
    E.add_global64 env "__stablemem_size" Mutable 0L;
    E.add_global64 env "__stablemem_version" Mutable version_stable_heap_no_regions

  let get_mem_size env =
    G.i (GlobalGet (nr (E.get_global env "__stablemem_size")))

  let set_mem_size env =
    G.i (GlobalSet (nr (E.get_global env "__stablemem_size")))

  let get_version env =
    G.i (GlobalGet (nr (E.get_global env "__stablemem_version")))

  let set_version env =
    G.i (GlobalSet (nr (E.get_global env "__stablemem_version")))

  let region_init env =
    compile_unboxed_const (if !Flags.use_stable_regions then 1L else 0L) ^^
    E.call_import env "rts" "region_init"

  (* stable memory bounds check *)
  let guard env =
       get_mem_size env ^^
       compile_unboxed_const (Int64.of_int page_size_bits) ^^
       G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)) ^^
       compile_comparison I64Op.GeU ^^
       E.then_trap_with env "StableMemory offset out of bounds"

  (* check both offset and [offset,.., offset + size) within bounds *)
  (* c.f. region.rs check_relative_range *)
  (* TODO: specialize on size *)
  let guard_range env =
      Func.share_code2 Func.Always env "__stablemem_guard_range"
        (("offset", I64Type), ("size", I64Type)) []
        (fun env get_offset get_size ->
          get_size ^^
          compile_unboxed_one ^^
          compile_comparison I64Op.LeU ^^
          E.if0 begin
            get_offset ^^
            guard env
          end
          begin
            compile_unboxed_const (Int64.minus_one) ^^
            get_size ^^
            G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
            get_offset ^^
            compile_comparison I64Op.LtU ^^
            E.then_trap_with env "StableMemory range overflow" ^^
            get_offset ^^
            get_size ^^
            G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
            get_mem_size env ^^
            compile_unboxed_const (Int64.of_int page_size_bits) ^^
            G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)) ^^
            compile_comparison I64Op.GtU ^^
            E.then_trap_with env "StableMemory range out of bounds"
          end)

  let add_guard env guarded get_offset bytes =
    if guarded then
     (get_offset ^^
      if bytes = 1L then
        guard env
      else
        compile_unboxed_const bytes ^^
        guard_range env)
    else G.nop

  (* TODO: crusso in read/write could avoid stack allocation by reserving and re-using scratch memory instead *)
  let read env guarded name typ bytes load =
      Func.share_code1 Func.Never env (Printf.sprintf "__stablemem_%sread_%s" (if guarded then "guarded_" else "") name)
        ("offset", I64Type) [typ]
        (fun env get_offset ->
          let words = Int64.div (Int64.add bytes 3L) 4L in
          add_guard env guarded get_offset bytes ^^
          Stack.with_words env "temp_ptr" words (fun get_temp_ptr ->
            get_temp_ptr ^^
            get_offset ^^
            compile_unboxed_const bytes ^^
            stable64_read env ^^
            get_temp_ptr ^^ load))

  let write env guarded name typ bytes store =
      Func.share_code2 Func.Never env (Printf.sprintf "__stablemem_%swrite_%s" (if guarded then "guarded_" else "") name)
        (("offset", I64Type), ("value", typ)) []
        (fun env get_offset get_value ->
          let words = Int64.div (Int64.add bytes 3L) 4L in
          add_guard env guarded get_offset bytes ^^
          Stack.with_words env "temp_ptr" words (fun get_temp_ptr ->
            get_temp_ptr ^^ get_value ^^ store ^^
            get_offset ^^
            get_temp_ptr ^^
            compile_unboxed_const bytes ^^
            stable64_write env))

  let load_word32 = G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = None})
  let store_word32 : G.t = G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None})

  let write_word32 env =
    write env false "word32" I32Type 4L store_word32

  let write_word64 env =
    write env false "word64" I64Type 8L store_unskewed_ptr

  let read_word32 env =
    read env false "word32" I32Type 4L load_word32

  let read_word64 env =
    read env false "word64" I64Type 8L load_unskewed_ptr

  (* ensure_pages : ensure at least num pages allocated,
     growing (real) stable memory if needed *)
  let ensure_pages env =
      Func.share_code1 Func.Always env "__stablemem_ensure_pages"
        ("pages", I64Type) [I64Type]
        (fun env get_pages ->
          let (set_size, get_size) = new_local env "size" in
          let (set_pages_needed, get_pages_needed) = new_local env "pages_needed" in

          stable64_size env ^^
          set_size ^^

          get_pages ^^
          get_size ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
          set_pages_needed ^^

          get_pages_needed ^^
          compile_unboxed_const 0L ^^
          compile_comparison I64Op.GtS ^^
          E.if1 I64Type
            (get_pages_needed ^^
             stable64_grow env)
            get_size)

      (* ensure stable memory includes [offset..offset+size), assumes size > 0 *)
  let ensure env =
      Func.share_code2 Func.Always env "__stablemem_ensure"
        (("offset", I64Type), ("size", I64Type)) []
        (fun env get_offset get_size ->
          let (set_sum, get_sum) = new_local env "sum" in
          get_offset ^^
          get_size ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
          set_sum ^^
          (* check for overflow *)
          get_sum ^^
          get_offset ^^
          compile_comparison I64Op.LtU ^^
          E.then_trap_with env "Range overflow" ^^
          (* ensure page *)
          get_sum ^^
          compile_unboxed_const (Int64.of_int page_size_bits) ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrU)) ^^
          compile_add_const 1L ^^
          ensure_pages env ^^
          (* Check result *)
          compile_unboxed_const 0L ^^
          compile_comparison I64Op.LtS ^^
          E.then_trap_with env "Out of stable memory.")

  (* low-level grow, respecting --max-stable-pages *)
  let grow env =
      Func.share_code1 Func.Always env "__stablemem_grow"
        ("pages", I64Type) [I64Type] (fun env get_pages ->
          let (set_size, get_size) = new_local env "size" in
          get_mem_size env ^^
          set_size ^^

          (* check within --max-stable-pages *)
          get_size ^^
          get_pages ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
          compile_unboxed_const (Int64.of_int (!Flags.max_stable_pages)) ^^
          compile_comparison I64Op.GtU ^^
          E.if1 I64Type
            begin
             compile_unboxed_const (-1L) ^^
             G.i Return
            end
            begin
              let (set_new_size, get_new_size) = new_local env "new_size" in
              get_size ^^
              get_pages ^^
              G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
              set_new_size ^^

              (* physical grow if necessary *)
              let (set_ensured, get_ensured) = new_local env "ensured" in
              get_new_size ^^
              ensure_pages env ^^
              set_ensured ^^

              (* Check result *)
              get_ensured ^^
              compile_unboxed_const 0L ^^
              compile_comparison I64Op.LtS ^^
              E.if1 I64Type
                ((* propagate failure -1; preserve logical size *)
                 get_ensured)
                ((* update logical size *)
                 get_new_size ^^
                 set_mem_size env ^^
                 (* return old logical size *)
                 get_size)
            end)

  let load_word32 env =
    read env true "word32" I32Type 4L
      (G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = None }))
  let store_word32 env =
    write env true "word32" I32Type 4L
      (G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None}))

  let load_word8 env =
    read env true "word8" I32Type 1L
      (G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack8, ZX)}))
  let store_word8 env =
    write env true "word8" I32Type 1L
      (G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None}))

  let load_word16 env =
    read env true "word16" I32Type 2L
      (G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.(Pack16, ZX)}))
  let store_word16 env =
    write env true "word16" I32Type 2L
      (G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None}))

  let load_word64 env =
    read env true "word64" I64Type 8L load_unskewed_ptr
  let store_word64 env =
    write env true "word64" I64Type 8L store_unskewed_ptr

  let load_float64 env =
    read env true "float64" F64Type 8L
      (G.i (Load {ty = F64Type; align = 0; offset = 0L; sz = None }))
  let store_float64 env =
    write env true "float64" F64Type 8L
      (G.i (Store {ty = F64Type; align = 0; offset = 0L; sz = None}))

  let load_blob env =
      Func.share_code2 Func.Always env "__stablemem_load_blob"
        (("offset", I64Type), ("len", I64Type)) [I64Type]
        (fun env get_offset get_len ->
          let (set_blob, get_blob) = new_local env "blob" in
          get_offset ^^
          get_len ^^
          guard_range env ^^
          Blob.alloc env Tagged.B get_len ^^ set_blob ^^
          get_blob ^^ Blob.payload_ptr_unskewed env ^^
          get_offset ^^
          get_len ^^
          stable64_read env ^^
          get_blob)

  let store_blob env =
      Func.share_code2 Func.Always env "__stablemem_store_blob"
        (("offset", I64Type), ("blob", I64Type)) []
        (fun env get_offset get_blob ->
         let (set_len, get_len) = new_local env "len" in
          get_blob ^^ Blob.len env ^^ set_len ^^
          get_offset ^^
          get_len ^^
          guard_range env ^^
          get_offset ^^
          get_blob ^^ Blob.payload_ptr_unskewed env ^^
          get_len ^^
          stable64_write env)

end (* StableMem *)

(* StableMemoryInterface *)
(* Core, legacy interface to IC stable memory, used to implement prims `stableMemoryXXX` of
   library `ExperimentalStableMemory.mo`.
   Each operation dispatches on the state of `StableMem.get_version()`.
   * StableMem.version_stable_heap_no_regions
     * use StableMem directly.
   * StableMem.version_stable_heap_regions: use Region.mo
*)
module StableMemoryInterface = struct

  (* Helpers *)
  let get_region0 env = E.call_import env "rts" "region0_get"

  let if_regions env args tys is1 is2 =
    StableMem.get_version env ^^
    compile_unboxed_const StableMem.version_stable_heap_regions ^^
    compile_comparison I64Op.Eq ^^
    E.if_ env tys
      (get_region0 env ^^ args ^^ is1 env)
      (args ^^ is2 env)

  (* Prims *)
  let size env =
    E.require_stable_memory env;
    Func.share_code0 Func.Always env "__stablememory_size" [I64Type]
      (fun env ->
        if_regions env
          G.nop
          [I64Type]
          Region.size
          StableMem.get_mem_size)

  let grow env =
    E.require_stable_memory env;
    Func.share_code1 Func.Always env "__stablememory_grow" ("pages", I64Type) [I64Type]
      (fun env get_pages ->
        if_regions env
          get_pages
          [I64Type]
          Region.grow
          (fun env ->
            (* logical grow *)
            StableMem.grow env))

  let load_blob env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_load_blob"
      (("offset", I64Type), ("len", I64Type)) [I64Type]
      (fun env offset len ->
        if_regions env
          (offset ^^ len)
          [I64Type]
          Region.load_blob
          StableMem.load_blob)
  let store_blob env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_store_blob"
      (("offset", I64Type), ("blob", I64Type)) []
      (fun env offset blob ->
        if_regions env
          (offset ^^ blob)
          []
          Region.store_blob
          StableMem.store_blob)

  let load_word8 env =
    E.require_stable_memory env;
    Func.share_code1 Func.Never env "__stablememory_load_word8"
      ("offset", I64Type) [I32Type]
      (fun env offset ->
        if_regions env
          offset
          [I32Type]
          Region.load_word8
          StableMem.load_word8)
  let store_word8 env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_store_word8"
      (("offset", I64Type), ("value", I32Type)) []
      (fun env offset value ->
        if_regions env
          (offset ^^ value)
          []
          Region.store_word8
          StableMem.store_word8)

  let load_word16 env =
    E.require_stable_memory env;
    Func.share_code1 Func.Never env "__stablememory_load_word16"
      ("offset", I64Type) [I32Type]
      (fun env offset->
        if_regions env
          offset
          [I32Type]
          Region.load_word16
          StableMem.load_word16)
  let store_word16 env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_store_word16"
      (("offset", I64Type), ("value", I32Type)) []
      (fun env offset value ->
        if_regions env
          (offset ^^ value)
          []
          Region.store_word16
          StableMem.store_word16)

  let load_word32 env =
    E.require_stable_memory env;
    Func.share_code1 Func.Never env "__stablememory_load_word32"
      ("offset", I64Type) [I32Type]
      (fun env offset ->
        if_regions env
          offset
          [I32Type]
          Region.load_word32
          StableMem.load_word32)
  let store_word32 env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_store_word32"
      (("offset", I64Type), ("value", I32Type)) []
      (fun env offset value ->
        if_regions env
          (offset ^^ value)
          []
          Region.store_word32
          StableMem.store_word32)

  let load_word64 env =
    E.require_stable_memory env;
    Func.share_code1 Func.Never env "__stablememory_load_word64" ("offset", I64Type) [I64Type]
      (fun env offset ->
        if_regions env
          offset
          [I64Type]
          Region.load_word64
          StableMem.load_word64)
  let store_word64 env =
    E.require_stable_memory env;
    Func.share_code2 Func.Never env "__stablememory_store_word64"
      (("offset", I64Type), ("value", I64Type)) []
      (fun env offset value ->
        if_regions env
          (offset ^^ value)
          []
          Region.store_word64
          StableMem.store_word64)

  let load_float64 env =
    E.require_stable_memory env;
    Func.share_code1 Func.Never env "__stablememory_load_float64"
      ("offset", I64Type) [F64Type]
      (fun env offset ->
        if_regions env
          offset
          [F64Type]
          Region.load_float64
          StableMem.load_float64)
  let store_float64 env =
    Func.share_code2 Func.Never env "__stablememory_store_float64"
      (("offset", I64Type), ("value", F64Type)) []
      (fun env offset value ->
        if_regions env
          (offset ^^ value)
          []
          Region.store_float64
          StableMem.store_float64)

end

module UpgradeStatistics = struct
  let get_upgrade_instructions env =
    E.call_import env "rts" "get_upgrade_instructions"
  let set_upgrade_instructions env =
    E.call_import env "rts" "set_upgrade_instructions"

  let add_instructions env =
    get_upgrade_instructions env ^^
    GC.instruction_counter env ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    set_upgrade_instructions env

  let set_instructions env =
    GC.instruction_counter env ^^
    set_upgrade_instructions env
end

module RTS_Exports = struct
  (* Must be called late, after main codegen, to ensure correct generation of
     of functioning or unused-but-trapping stable memory exports (as required)
   *)
  let system_exports env =

    (* Value constructors *)

    let int_from_i64_fi = E.add_fun env "int_from_i64" (
      Func.of_body env ["v", I64Type] [I64Type] (fun env ->
        let get_v = G.i (LocalGet (nr 0l)) in
        get_v ^^ BigNum.from_signed_word64 env
      )
    ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "int_from_i64";
      edesc = nr (FuncExport (nr int_from_i64_fi))
    });

    (* Traps *)

    let bigint_trap_fi = E.add_fun env "bigint_trap" (
      Func.of_body env [] [] (fun env ->
        E.trap_with env "bigint function error"
      )
    ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "bigint_trap";
      edesc = nr (FuncExport (nr bigint_trap_fi))
    });

    (* Keep a memory reserve when in update or init state.
       This reserve can be used by queries, composite queries, and upgrades. *)
    let keep_memory_reserve_fi = E.add_fun env "keep_memory_reserve" (
      Func.of_body env [] [I32Type] (fun env ->
        Lifecycle.get env ^^
        compile_eq_const Lifecycle.(int_of_state InUpdate) ^^
        Lifecycle.get env ^^
        compile_eq_const Lifecycle.(int_of_state InInit) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        Bool.to_rts_int32
      )
    ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "keep_memory_reserve";
      edesc = nr (FuncExport (nr keep_memory_reserve_fi))
    });

    let rts_trap_fi = E.add_fun env "rts_trap" (
      (* `libc` still uses 32-bit length parameter for `rts_trap` *)
      Func.of_body env ["str", I64Type; "len", I32Type] [] (fun env ->
        let get_str = G.i (LocalGet (nr 0l)) in
        let get_len = G.i (LocalGet (nr 1l)) in
        get_str ^^
        get_len ^^
        G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
        IC.trap_ptr_len env
      )
    ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "rts_trap";
      edesc = nr (FuncExport (nr rts_trap_fi))
    });

    let ic0_performance_counter_fi =
      if E.mode env = Flags.WASIMode then
        E.add_fun env "ic0_performance_counter" (
            Func.of_body env ["number", I32Type] [I64Type]
              (fun env ->
                E.trap_with env "ic0_performance_counter is not supposed to be called in WASI"
              )
          )
      else E.reuse_import env "ic0" "performance_counter" in
    E.add_export env (nr {
      name = Lib.Utf8.decode "ic0_performance_counter";
      edesc = nr (FuncExport (nr ic0_performance_counter_fi))
    });

    (* Keep a memory reserve when in update or init state.
    This reserve can be used by queries, composite queries, and (graph-copy) upgrades. *)
    let keep_memory_reserve_fi = E.add_fun env "keep_memory_reserve" (
      Func.of_body env [] [I32Type] (fun env ->
        Lifecycle.(is_in int_of_state InUpdate) ^^
        Lifecycle.(is_in int_of_state InInit) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        Bool.to_rts_int32
      )
    ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "keep_memory_reserve";
      edesc = nr (FuncExport (nr keep_memory_reserve_fi))
    });

    let when_stable_memory_required_else_trap env code =
      if E.requires_stable_memory env then
        code() else
        E.trap_with env "unreachable" in

    let ic0_stable64_write_fi =
      match E.mode env with
      | Flags.ICMode | Flags.RefMode ->
        E.reuse_import env "ic0" "stable64_write"
      | Flags.WASIMode | Flags.WasmMode ->
        E.add_fun env "ic0_stable64_write" (
          Func.of_body env ["offset", I64Type; "src", I64Type; "size", I64Type] []
            (fun env ->
              when_stable_memory_required_else_trap env (fun () ->
               let get_offset = G.i (LocalGet (nr 0l)) in
               let get_src = G.i (LocalGet (nr 1l)) in
               let get_size = G.i (LocalGet (nr 2l)) in
               get_offset ^^
               get_src ^^
               get_size ^^
               StableMem.stable64_write env))
          )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "ic0_stable64_write";
      edesc = nr (FuncExport (nr ic0_stable64_write_fi))
    });

    let ic0_stable64_read_fi =
      match E.mode env with
      | Flags.ICMode | Flags.RefMode ->
        E.reuse_import env "ic0" "stable64_read"
      | Flags.WASIMode | Flags.WasmMode ->
        E.add_fun env "ic0_stable64_read" (
          Func.of_body env ["dst", I64Type; "offset", I64Type; "size", I64Type] []
            (fun env ->
              when_stable_memory_required_else_trap env (fun () ->
              let get_dst = G.i (LocalGet (nr 0l)) in
              let get_offset = G.i (LocalGet (nr 1l)) in
              let get_size = G.i (LocalGet (nr 2l)) in
              get_dst ^^
              get_offset ^^
              get_size ^^
              StableMem.stable64_read env))
          )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "ic0_stable64_read";
      edesc = nr (FuncExport (nr ic0_stable64_read_fi))
    });

    let ic0_stable64_size_fi =
      match E.mode env with
      | Flags.ICMode | Flags.RefMode ->
        E.reuse_import env "ic0" "stable64_size"
      | Flags.WASIMode | Flags.WasmMode ->
        E.add_fun env "ic0_stable64_size" (
          Func.of_body env [] [I64Type]
            (fun env ->
              if E.requires_stable_memory env then
                StableMem.stable64_size env
              else
                (* The RTS also checks the persistence mode on Wasi without stable memory support *)
                compile_unboxed_const 0L
            )
          )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "ic0_stable64_size";
      edesc = nr (FuncExport (nr ic0_stable64_size_fi))
    });

    let ic0_stable64_grow_fi =
      match E.mode env with
      | Flags.ICMode | Flags.RefMode ->
        E.reuse_import env "ic0" "stable64_grow"
      | Flags.WASIMode | Flags.WasmMode ->
        E.add_fun env "ic0_stable64_grow" (
          Func.of_body env ["newPages", I64Type] [I64Type]
            (fun env ->
              when_stable_memory_required_else_trap env (fun () ->
                G.i (LocalGet (nr 0l)) ^^
                StableMem.stable64_grow env))
          )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "ic0_stable64_grow";
      edesc = nr (FuncExport (nr ic0_stable64_grow_fi))
    });

    let moc_stable_mem_grow_fi =
      E.add_fun env "moc_stable_mem_grow" (
        Func.of_body env ["newPages", I64Type] [I64Type]
          (fun env ->
            when_stable_memory_required_else_trap env (fun () ->
            G.i (LocalGet (nr 0l)) ^^
            StableMem.grow env))
        )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_mem_grow";
      edesc = nr (FuncExport (nr moc_stable_mem_grow_fi))
    });

    let moc_stable_mem_get_size_fi =
      E.add_fun env "moc_stable_mem_get_size" (
        Func.of_body env [] [I64Type]
          (fun env ->
             when_stable_memory_required_else_trap env (fun () ->
             StableMem.get_mem_size env))
        )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_mem_get_size";
      edesc = nr (FuncExport (nr moc_stable_mem_get_size_fi))
    });

    let moc_stable_mem_set_size_fi =
      E.add_fun env "moc_stable_mem_set_size" (
        Func.of_body env ["pages", I64Type] []
          (fun env ->
            match E.mode env with
            | Flags.ICMode | Flags.RefMode ->
               G.i (LocalGet (nr 0l)) ^^
               StableMem.set_mem_size env
            | _ ->
               E.trap_with env "moc_stable_mem_set_size is not supposed to be called in WASI" (* improve me *)
          )
        )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_mem_set_size";
      edesc = nr (FuncExport (nr moc_stable_mem_set_size_fi))
    });

    let moc_stable_mem_get_version_fi =
      E.add_fun env "moc_stable_mem_get_version" (
        Func.of_body env [] [I64Type]
          (fun env ->
             StableMem.get_version env)
        )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_mem_get_version";
      edesc = nr (FuncExport (nr moc_stable_mem_get_version_fi))
    });

    let moc_stable_mem_set_version_fi =
      E.add_fun env "moc_stable_mem_set_version" (
        Func.of_body env ["version", I64Type] []
          (fun env ->
             G.i (LocalGet (nr 0l)) ^^
             StableMem.set_version env
          )
        )
    in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_mem_set_version";
      edesc = nr (FuncExport (nr moc_stable_mem_set_version_fi))
      });

    E.add_export env (nr {
        name = Lib.Utf8.decode "idl_limit_check";
        edesc = nr (FuncExport (nr (E.built_in env "idl_limit_check")))
      })

end (* RTS_Exports *)

module StackRep = struct
  open SR

  (*
     Most expressions have a “preferred”, most optimal, form. Hence,
     compile_exp put them on the stack in that form, and also returns
     the form it chose.

     But the users of compile_exp usually want a specific form as well.
     So they use compile_exp_as, indicating the form they expect.
     compile_exp_as then does the necessary coercions.
   *)

  let of_arity n =
    if n = 1 then Vanilla else UnboxedTuple n

  (* The stack rel of a primitive type, i.e. what the binary operators expect *)
  let of_type t =
    let open Type in
    match normalize t with
    | Prim Bool -> SR.bool
    | Prim (Nat | Int) -> Vanilla
    | Prim ((Nat8 | Nat16 | Nat32 | Nat64 | Int8 | Int16 | Int32 | Int64 | Char) as pty) -> UnboxedWord64 pty
    | Prim (Text | Blob | Principal) -> Vanilla
    | Prim Float -> UnboxedFloat64
    | Obj (Actor, _) -> Vanilla
    | Func (Shared _, _, _, _, _) -> Vanilla
    | p -> todo "StackRep.of_type" (Arrange_ir.typ p) Vanilla

  (* The env looks unused, but will be needed once we can use multi-value, to register
     the complex types in the environment.
     For now, multi-value block returns are handled via FakeMultiVal. *)
  let to_block_type env = function
    | Vanilla -> [I64Type]
    | UnboxedWord64 _ -> [I64Type]
    | UnboxedFloat64 -> [F64Type]
    | UnboxedTuple n -> Lib.List.make n I64Type
    | Const _ -> []
    | Unreachable -> []

  let to_string = function
    | Vanilla -> "Vanilla"
    | UnboxedWord64 pty -> prim_fun_name pty "UnboxedWord64"
    | UnboxedFloat64 -> "UnboxedFloat64"
    | UnboxedTuple n -> Printf.sprintf "UnboxedTuple %d" n
    | Unreachable -> "Unreachable"
    | Const _ -> "Const"

  let join (sr1 : t) (sr2 : t) = match sr1, sr2 with
    | _, _ when SR.eq sr1 sr2 -> sr1
    | Unreachable, sr2 -> sr2
    | sr1, Unreachable -> sr1

    | Const _, Const _ -> Vanilla
    | Const _, sr2_ -> sr2
    | sr1, Const _ -> sr1

    | _, Vanilla -> Vanilla
    | Vanilla, _ -> Vanilla

    | UnboxedTuple n, UnboxedTuple m when n = m -> sr1

    | _, _ ->
      Printf.eprintf "Invalid stack rep join (%s, %s)\n"
        (to_string sr1) (to_string sr2); sr1

  let joins = List.fold_left join Unreachable

  let drop env (sr_in : t) =
    match sr_in with
    | Vanilla | UnboxedWord64 _ | UnboxedFloat64 -> G.i Drop
    | UnboxedTuple n -> G.table n (fun _ -> G.i Drop)
    | Const _ | Unreachable -> G.nop

  let rec build_constant env constant =
    E.constant_pool_add env constant (fun () -> build_constant_aux env constant)

  and build_constant_aux env = function
  | Const.Lit (Const.Vanilla value) -> E.Vanilla value
  | Const.Lit (Const.Bool number) -> E.Vanilla (Bool.vanilla_lit number)
  | Const.Lit (Const.Text payload) -> Blob.constant env Tagged.T payload
  | Const.Lit (Const.Blob payload) -> Blob.constant env Tagged.B payload
  | Const.Lit (Const.Null) -> E.Vanilla Opt.null_vanilla_lit
  | Const.Lit (Const.BigInt number) -> BigNum.constant env number
  | Const.Lit (Const.Word64 (pty, number)) -> BoxedWord64.constant env pty number
  | Const.Lit (Const.Float64 number) -> Float.constant env number
  | Const.Opt value -> Opt.constant env (build_constant env value)
  | Const.Fun (_, get_fi, _) -> Closure.constant env get_fi
  | Const.Message _ -> assert false
  | Const.Unit -> E.Vanilla (Tuple.unit_vanilla_lit env)
  | Const.Tag (tag, value) ->
      let payload = build_constant env value in
      Tagged.shared_object __LINE__ env (fun env ->
        let materialized_payload = Tagged.materialize_shared_value env payload in
        Variant.inject env tag materialized_payload
      )
  | Const.Array elements ->
      let constant_elements = List.map (build_constant env) elements in
      Arr.constant env Tagged.I constant_elements
  | Const.Tuple elements ->
      let constant_elements = List.map (build_constant env) elements in
      Arr.constant env Tagged.T constant_elements
  | Const.Obj fields ->
      let constant_fields = List.map (fun (name, value) -> (name, build_constant env value)) fields  in
      Object.constant env constant_fields

  let materialize_constant env value =
    Tagged.materialize_shared_value env (build_constant env value)

  let adjust env (sr_in : t) sr_out =
    if eq sr_in sr_out
    then G.nop
    else match sr_in, sr_out with
    | Unreachable, Unreachable -> G.nop
    | Unreachable, _ -> G.i Unreachable

    | UnboxedTuple n, Vanilla -> Tuple.from_stack env n
    | Vanilla, UnboxedTuple n -> Tuple.to_stack env n

    (* BoxedWord64 types *)
    | UnboxedWord64 (Type.(Int64 | Nat64) as pty), Vanilla ->
      BoxedWord64.box env pty
    | Vanilla, UnboxedWord64 (Type.(Int64 | Nat64) as pty) ->
      BoxedWord64.unbox env pty

    (* TaggedSmallWord types *)
    | UnboxedWord64 (Type.(Int8 | Nat8 | Int16 | Nat16 | Int32 | Nat32 | Char) as pty), Vanilla ->
      TaggedSmallWord.tag env pty
    | Vanilla, UnboxedWord64 (Type.(Nat8 | Int8 | Nat16 | Int16 | Int32 | Nat32 | Char) as pty) ->
      TaggedSmallWord.untag env pty

    | UnboxedFloat64, Vanilla -> Float.box env
    | Vanilla, UnboxedFloat64 -> Float.unbox env

    | Const value, Vanilla ->
        materialize_constant env value
    | Const Const.Lit (Const.Vanilla n), UnboxedWord64 ty ->
        compile_unboxed_const n ^^
        TaggedSmallWord.untag env ty
    | Const Const.Lit (Const.Word64 (ty1, n)), UnboxedWord64 ty2 when ty1 = ty2 ->
        compile_unboxed_const n
    | Const Const.Lit (Const.Float64 f), UnboxedFloat64 -> Float.compile_unboxed_const f
    | Const c, UnboxedTuple 0 -> G.nop
    | Const Const.Tuple cs, UnboxedTuple n ->
      assert (n = List.length cs);
      G.concat_map (fun c -> materialize_constant env c) cs
    | _, _ ->
      Printf.eprintf "Unknown stack_rep conversion %s -> %s\n"
        (to_string sr_in) (to_string sr_out);
      assert false

end (* StackRep *)

module VarEnv = struct

  (* A type to record where Motoko names are stored. *)
  type varloc =
    (* A Wasm Local of the current function, directly containing the value,
       in the given stackrep (Vanilla, UnboxedWord64, …) so far
       Used for immutable and mutable, non-captured data *)
    | Local of SR.t * int32
    (* A Wasm Local of the current function, that points to memory location,
       which is a MutBox.  Used for mutable captured data *)
    | HeapInd of int32
    (* A static variable accessed by an index via the runtime system, refers to a MutBox,
       belonging to the GC root set *)
    | Static of int64
    (* Constant literals can reside in dynamic heap *)
    | Const of Const.v
    (* public method *)
    | PublicMethod of int32 * string

  let is_non_local : varloc -> bool = function
    | Local _
    | HeapInd _ -> false
    | Static _
    | PublicMethod _
    | Const _ -> true

  type lvl = TopLvl | NotTopLvl

  (*
  The source variable environment:
   - Whether we are on the top level
   - In-scope variables
   - scope jump labels
  *)


  module NameEnv = Env.Make(String)
  type t = {
    lvl : lvl;
    vars : (varloc * Type.typ) NameEnv.t; (* variables ↦ their location and type *)
    labels : G.depth NameEnv.t; (* jump label ↦ their depth *)
  }

  let empty_ae = {
    lvl = TopLvl;
    vars = NameEnv.empty;
    labels = NameEnv.empty;
  }

  (* Creating a local environment, resetting the local fields,
     and removing bindings for local variables (unless they are at global locations)
  *)

  let mk_fun_ae ae = { ae with
    lvl = NotTopLvl;
    vars = NameEnv.filter (fun v (l, _) ->
      let non_local = is_non_local l in
      (* For debugging, enable this:
      (if not non_local then Printf.eprintf "VarEnv.mk_fun_ae: Removing %s\n" v);
      *)
      non_local
    ) ae.vars;
  }
  let lookup ae var =
    match NameEnv.find_opt var ae.vars with
      | Some e -> Some e
      | None   -> Printf.eprintf "Could not find %s\n" var; None

  let lookup_var ae var =
    match lookup ae var with
      | Some (l, _) -> Some l
      | None -> None

  let needs_capture ae var = match lookup_var ae var with
    | Some l -> not (is_non_local l)
    | None -> assert false

  let add_local_with_heap_ind env (ae : t) name typ =
      let i = E.add_anon_local env I64Type in
      E.add_local_name env i name;
      ({ ae with vars = NameEnv.add name ((HeapInd i), typ) ae.vars }, i)

  let add_static_variable (ae : t) name index typ =
      { ae with vars = NameEnv.add name ((Static index), typ) ae.vars }

  let add_local_public_method (ae : t) name (fi, exported_name) typ =
      { ae with vars = NameEnv.add name ((PublicMethod (fi, exported_name) : varloc), typ) ae.vars }

  let add_local_const (ae : t) name cv typ =
      { ae with vars = NameEnv.add name ((Const cv : varloc), typ) ae.vars }

  let add_local_local env (ae : t) name sr i typ =
      { ae with vars = NameEnv.add name ((Local (sr, i)), typ) ae.vars }

  let add_direct_local env (ae : t) name sr typ =
      let i = E.add_anon_local env (SR.to_var_type sr) in
      E.add_local_name env i name;
      (add_local_local env ae name sr i typ, i)

  (* Adds the names to the environment and returns a list of setters *)
  let rec add_arguments env (ae : t) as_local = function
    | [] -> ae
    | ((name, typ) :: remainder) ->
      if as_local name then
        let i = E.add_anon_local env I64Type in
        E.add_local_name env i name;
        let ae' = { ae with vars = NameEnv.add name ((Local (SR.Vanilla, i)), typ) ae.vars } in
        add_arguments env ae' as_local remainder
      else
        let index = MutBox.add_global_mutbox env in
        let ae' = add_static_variable ae name index typ in
        add_arguments env ae' as_local remainder

  let add_argument_locals env (ae : t) =
    add_arguments env ae (fun _ -> true)

  let add_label (ae : t) name (d : G.depth) =
      { ae with labels = NameEnv.add name d ae.labels }

  let get_label_depth (ae : t) name : G.depth  =
    match NameEnv.find_opt name ae.labels with
      | Some d -> d
      | None   -> raise (CodegenError (Printf.sprintf "Could not find %s\n" name))

end (* VarEnv *)

(* type for wrapping code with context, context is establishment
   of (pattern) binding, argument is the code using the binding,
   result is e.g. the code for `case p e`. *)
type scope_wrap = G.t -> G.t

let unmodified : scope_wrap = fun code -> code

let rec can_be_pointer typ nested_optional =
  Type.(match normalize typ with
  | Mut t -> (can_be_pointer t nested_optional)
  | Opt t -> (if nested_optional then true else (can_be_pointer t true))
  | Prim (Null| Bool | Char | Nat8 | Nat16 | Int8 | Int16) | Non | Tup [] -> false
  | _ -> true)

let potential_pointer typ : bool =
  (* must not eliminate nested optional types as they refer to a heap object for ??null, ???null etc. *)
  can_be_pointer typ false

module Var = struct
  (* This module is all about looking up Motoko variables in the environment,
     and dealing with mutable variables *)

  open VarEnv

  (* Returns desired stack representation, preparation code and code to consume
     the value onto the stack *)
  let set_val env ae var : G.t * SR.t * G.t = match VarEnv.lookup ae var with
    | Some ((Local (sr, i)), _) ->
      G.nop,
      sr,
      G.i (LocalSet (nr i))
    | Some ((HeapInd i), typ) when potential_pointer typ ->
      G.i (LocalGet (nr i)) ^^
      Tagged.load_forwarding_pointer env ^^
      compile_add_const ptr_unskew ^^
      compile_add_const (Int64.mul MutBox.field Heap.word_size),
      SR.Vanilla,
      Tagged.write_with_barrier env
    | Some ((HeapInd i), typ) ->
      G.i (LocalGet (nr i)),
      SR.Vanilla,
      MutBox.store_field env
    | Some ((Static index), typ) when potential_pointer typ ->
      Heap.get_static_variable env index ^^
      Tagged.load_forwarding_pointer env ^^
      compile_add_const ptr_unskew ^^
      compile_add_const (Int64.mul MutBox.field Heap.word_size),
      SR.Vanilla,
      Tagged.write_with_barrier env
    | Some ((Static index), typ) ->
      Heap.get_static_variable env index,
      SR.Vanilla,
      MutBox.store_field env
    | Some ((Const _), _) -> fatal "set_val: %s is const" var
    | Some ((PublicMethod _), _) -> fatal "set_val: %s is PublicMethod" var
    | None -> fatal "set_val: %s missing" var

  (* Stores the payload. Returns stack preparation code, and code that consumes the values from the stack *)
  let set_val_vanilla env ae var : G.t * G.t =
    let pre_code, sr, code = set_val env ae var in
    pre_code, StackRep.adjust env SR.Vanilla sr ^^ code

  (* Stores the payload (which is found on the stack, in Vanilla stackrep) *)
  let set_val_vanilla_from_stack env ae var : G.t =
    let pre_code, code = set_val_vanilla env ae var in
    if G.is_nop pre_code
    then code
    else
      (* Need to shuffle the stack entries *)
      let (set_x, get_x) = new_local env "var_scrut" in
      set_x ^^
      pre_code ^^
      get_x ^^
      code

  (* Returns the payload (optimized representation) *)
  let get_val (env : E.t) (ae : VarEnv.t) var = match VarEnv.lookup_var ae var with
    | Some (Local (sr, i)) ->
      sr, G.i (LocalGet (nr i))
    | Some (HeapInd i) ->
      SR.Vanilla, G.i (LocalGet (nr i)) ^^ MutBox.load_field env
    | Some (Static index) ->
      SR.Vanilla,
      Heap.get_static_variable env index ^^
      MutBox.load_field env
    | Some (Const c) ->
      SR.Const c, G.nop
    | Some (PublicMethod (_, name)) ->
      SR.Vanilla,
      IC.get_self_reference env ^^
      IC.actor_public_field env name
    | None -> assert false

  (* Returns the payload (vanilla representation) *)
  let get_val_vanilla (env : E.t) (ae : VarEnv.t) var =
    let sr, code = get_val env ae var in
    code ^^ StackRep.adjust env sr SR.Vanilla

  (* Returns the value to put in the closure,
     and code to restore it, including adding to the environment
  *)
  let capture old_env ae0 var : G.t * (E.t -> VarEnv.t -> VarEnv.t * scope_wrap) =
    match VarEnv.lookup ae0 var with
    | Some ((Local (sr, i)), typ) ->
      ( G.i (LocalGet (nr i)) ^^ StackRep.adjust old_env sr SR.Vanilla
      , fun new_env ae1 ->
        (* we use SR.Vanilla in the restored environment. We could use sr;
           like for parameters hard to predict what’s better *)
        let ae2, j = VarEnv.add_direct_local new_env ae1 var SR.Vanilla typ in
        let restore_code = G.i (LocalSet (nr j))
        in ae2, fun body -> restore_code ^^ body
      )
    | Some ((HeapInd i), typ) ->
      ( G.i (LocalGet (nr i))
      , fun new_env ae1 ->
        let ae2, j = VarEnv.add_local_with_heap_ind new_env ae1 var typ in
        let restore_code = G.i (LocalSet (nr j))
        in ae2, fun body -> restore_code ^^ body
      )
    | _ -> assert false

  (* This is used when putting a mutable field into an object.
     In the IR, mutable fields of objects are pre-allocated as MutBox objects,
     to allow the async/await.
     So we expect the variable to be in a HeapInd (pointer to MutBox on the heap),
     or Static (static variable represented as a MutBox that is accessed via the
     runtime system) and we use the pointer.
  *)
  let get_aliased_box env ae var = match VarEnv.lookup_var ae var with
    | Some (HeapInd i) -> G.i (LocalGet (nr i))
    | Some (Static index) -> Heap.get_static_variable env index
    | _ -> assert false

  let capture_aliased_box env ae var = match VarEnv.lookup_var ae var with
    | Some (HeapInd i) ->
      G.i (LocalSet (nr i))
    | _ -> assert false

end (* Var *)

(* Calling well-known prelude functions *)
(* FIXME: calling into the prelude will not work if we ever need to compile a program
that requires top-level cps conversion;
use new prims instead *)
module Internals = struct
  let call_prelude_function env ae var =
    match VarEnv.lookup_var ae var with
    | Some (VarEnv.Const Const.Fun (_, mk_fi, _)) ->
        compile_unboxed_zero ^^ (* A dummy closure *)
        G.i (Call (nr (mk_fi())))
    | _ -> assert false

  let add_cycles env ae = call_prelude_function env ae "@add_cycles"
  let reset_cycles env ae = call_prelude_function env ae "@reset_cycles"
  let reset_refund env ae = call_prelude_function env ae "@reset_refund"

  let register_dedup env ae =
    match VarEnv.lookup_var ae "@dedup" with
    | Some (VarEnv.Const Const.Fun (_, mk_fi, _)) ->
      E.set_dedup env mk_fi
    | _ -> assert false

  let dedup env = G.i (Call (nr (E.get_dedup env)))

end

module Serialization = struct
  (*
    The general serialization strategy is as follows:
    * We statically generate the IDL type description header.
    * We traverse the data to calculate the size needed for the data buffer and the
      reference buffer.
    * We allocate memory for the data buffer and the reference buffer
      (this memory area is not referenced, so will be dead with the next GC)
    * We copy the IDL type header to the data buffer.
    * We traverse the data and serialize it into the data buffer.
      This is type driven, and we use the `share_code` machinery and names that
      properly encode the type to resolve loops in a convenient way.
    * We externalize all that new data space into a databuf
    * We externalize the reference space into a elembuf
    * We pass both databuf and elembuf to shared functions
      (this mimicks the future system API)

    The deserialization is analogous:
    * We allocate some scratch space, and internalize the databuf and elembuf into it.
    * We parse the data, in a type-driven way, using normal construction and
      allocation, while keeping tabs on the type description header for subtyping.
    * At the end, the scratch space is a hole in the heap, and will be reclaimed
      by the next GC.
  *)

  module Strm = struct
    (* Creates a fresh stream with header, storing stream token. *)
    let create env get_data_size set_data_buf get_data_buf header =
      let header_size = Int64.of_int (String.length header) in
      get_data_size ^^ compile_add_const header_size ^^
      Blob.dyn_alloc_scratch env ^^ set_data_buf ^^
      get_data_buf ^^
      Blob.lit env Tagged.B header ^^ Blob.payload_ptr_unskewed env ^^
      compile_unboxed_const header_size ^^
      Heap.memcpy env ^^
      get_data_buf ^^ compile_add_const header_size ^^ set_data_buf

    (* Checks the stream's filling, traps if unexpected *)
    let check_filled env get_data_buf get_data_size =
      get_data_buf ^^ get_data_size ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
      compile_comparison I64Op.Eq ^^
      E.else_trap_with env "data buffer not filled"

    (* Finishes the stream, performing consistency checks.
      Returns payload address and size including the header. *)
    let terminate env get_data_buf get_data_size header_size =
      get_data_buf ^^ compile_sub_const header_size ^^
      get_data_size ^^ compile_add_const header_size

    (* Builds a unique name for a name seed and a type. *)
    let name_for fn_name ts = "@" ^ fn_name ^ "<" ^ Typ_hash.typ_seq_hash ts ^ ">"

    let advance_data_buf get_data_buf =
      get_data_buf ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^ G.setter_for get_data_buf

    (* Pushes the stream's current absolute byte offset on stack.
      The requirement is that the difference between two uses
      of this method must give a correct _relative_ offset. *)
    let absolute_offset _env get_data_buf = get_data_buf

    (* Opportunity to flush or update the token. Stream token is on stack. *)
    let checkpoint _env get_data_buf = G.setter_for get_data_buf

    (* Reserve a small fixed number of bytes in the stream and return an
       address to it. The address is invalidated by a GC, and as such must
       be written to in the next few instructions. *)
    let reserve _env get_data_buf bytes =
      get_data_buf ^^ get_data_buf ^^ compile_add_const bytes ^^ G.setter_for get_data_buf

    let write_word_leb env get_data_buf code =
      let set_word, get_word = new_local env "word" in
      code ^^ set_word ^^
      I32Leb.compile_store_to_data_buf_unsigned env get_word get_data_buf ^^
      advance_data_buf get_data_buf

    let write_word_32 env get_data_buf code =
      let word32_size = 4L in
      get_data_buf ^^ code ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
      G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None}) ^^
      compile_unboxed_const word32_size ^^
      advance_data_buf get_data_buf

    let write_byte _env get_data_buf code =
      get_data_buf ^^ code ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
      G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.Pack8}) ^^
      compile_unboxed_one ^^ advance_data_buf get_data_buf

    let write_blob env get_data_buf get_x =
      let set_len, get_len = new_local env "len" in
      get_x ^^ Blob.len env ^^ set_len ^^
      write_word_leb env get_data_buf get_len ^^
      get_data_buf ^^
      get_x ^^ Blob.payload_ptr_unskewed env ^^
      get_len ^^
      Heap.memcpy env ^^
      get_len ^^ advance_data_buf get_data_buf

    let write_text env get_data_buf get_x =
      let set_len, get_len = new_local env "len" in
      get_x ^^ Text.size env ^^ set_len ^^
      write_word_leb env get_data_buf get_len ^^
      get_x ^^ get_data_buf ^^ Text.to_buf env ^^
      get_len ^^ advance_data_buf get_data_buf

    let write_bignum_leb env get_data_buf get_x =
      get_data_buf ^^
      get_x ^^
      BigNum.compile_store_to_data_buf_unsigned env ^^
      advance_data_buf get_data_buf

    let write_bignum_sleb env get_data_buf get_x =
      get_data_buf ^^
      get_x ^^
      BigNum.compile_store_to_data_buf_signed env ^^
      advance_data_buf get_data_buf
  end (* Strm *)
  (* Globals recording known Candid types
    See Note [Candid subtype checks]
  *)

  let register_delayed_globals env =
    (E.add_global64_delayed env "__candid_data_length" Immutable,
    E.add_global64_delayed env "__type_offsets_length" Immutable,
    E.add_global64_delayed env "__idl_types_length" Immutable)

  let get_candid_data_length env =
    G.i (GlobalGet (nr (E.get_global env "__candid_data_length")))
  let get_type_offsets_length env =
    G.i (GlobalGet (nr (E.get_global env "__type_offsets_length")))
  let get_idl_types_length env =
    G.i (GlobalGet (nr (E.get_global env "__idl_types_length")))

  let candid_type_offset_size = 8L

  let get_global_type_descriptor env =
    match !(E.(env.global_type_descriptor)) with
    | Some descriptor -> descriptor
    | None -> assert false

  let get_global_candid_data env =
    Tagged.share env (fun env ->
      let descriptor = get_global_type_descriptor env in
      Blob.load_data_segment env Tagged.B E.(descriptor.candid_data_segment) (get_candid_data_length env)
    )

  let get_global_type_offsets env =
    Tagged.share env (fun env ->
      let descriptor = get_global_type_descriptor env in
      Blob.load_data_segment env Tagged.B E.(descriptor.type_offsets_segment) (get_type_offsets_length env)
    )

  let get_global_idl_types env =
    Tagged.share env (fun env ->
      let descriptor = get_global_type_descriptor env in
      Blob.load_data_segment env Tagged.B E.(descriptor.idl_types_segment) (get_idl_types_length env)
    )

  module Registers = struct

    (* interval for checking instruction counter *)
    let idl_value_numerator = 1L
    let idl_value_denominator = 1L
    let idl_value_bias = 1024L
    let idl_typetbl_scaler = 16L
    let idl_typetbl_bias = 1024L

    let register_globals env =
      E.add_global64 env "@@rel_buf_opt" Mutable 0L;
      E.add_global64 env "@@data_buf" Mutable 0L;
      E.add_global64 env "@@ref_buf" Mutable 0L;
      E.add_global64 env "@@typtbl" Mutable 0L;
      E.add_global64 env "@@typtbl_end" Mutable 0L;
      E.add_global64 env "@@typtbl_size" Mutable 0L;
      E.add_global64 env "@@global_typtbl" Mutable 0L;
      E.add_global64 env "@@global_typtbl_end" Mutable 0L;
      E.add_global64 env "@@global_typtbl_size" Mutable 0L;
      E.add_global64 env "@@value_denominator" Mutable idl_value_denominator;
      E.add_global64 env "@@value_numerator" Mutable idl_value_numerator;
      E.add_global64 env "@@value_bias" Mutable idl_value_bias;
      E.add_global64 env "@@value_quota" Mutable 0L;
      E.add_global64 env "@@type_scaler" Mutable idl_typetbl_scaler;
      E.add_global64 env "@@type_bias" Mutable idl_typetbl_bias

    let get_rel_buf_opt env =
      G.i (GlobalGet (nr (E.get_global env "@@rel_buf_opt")))
    let set_rel_buf_opt env =
      G.i (GlobalSet (nr (E.get_global env "@@rel_buf_opt")))

    let get_data_buf env =
      G.i (GlobalGet (nr (E.get_global env "@@data_buf")))
    let set_data_buf env =
      G.i (GlobalSet (nr (E.get_global env "@@data_buf")))

    let get_ref_buf env =
      G.i (GlobalGet (nr (E.get_global env "@@ref_buf")))
    let set_ref_buf env =
      G.i (GlobalSet (nr (E.get_global env "@@ref_buf")))

    let get_typtbl env =
      G.i (GlobalGet (nr (E.get_global env "@@typtbl")))
    let set_typtbl env =
      G.i (GlobalSet (nr (E.get_global env "@@typtbl")))

    let get_typtbl_end env =
      G.i (GlobalGet (nr (E.get_global env "@@typtbl_end")))
    let set_typtbl_end env =
      G.i (GlobalSet (nr (E.get_global env "@@typtbl_end")))

    let get_typtbl_size env =
      G.i (GlobalGet (nr (E.get_global env "@@typtbl_size")))
    let set_typtbl_size env =
      G.i (GlobalSet (nr (E.get_global env "@@typtbl_size")))

    let get_global_typtbl env =
      G.i (GlobalGet (nr (E.get_global env "@@global_typtbl")))
    let set_global_typtbl env =
      G.i (GlobalSet (nr (E.get_global env "@@global_typtbl")))

    let get_global_typtbl_end env =
      G.i (GlobalGet (nr (E.get_global env "@@global_typtbl_end")))
    let set_global_typtbl_end env =
      G.i (GlobalSet (nr (E.get_global env "@@global_typtbl_end")))

    let get_global_typtbl_size env =
      G.i (GlobalGet (nr (E.get_global env "@@global_typtbl_size")))
    let set_global_typtbl_size env =
      G.i (GlobalSet (nr (E.get_global env "@@global_typtbl_size")))

    (* Used as safety guard that no temporary pointers remain in the registers across GC increments. *)
    let clear_registers env =
      compile_unboxed_const 0L ^^ set_rel_buf_opt env ^^
      compile_unboxed_const 0L ^^ set_data_buf env ^^
      compile_unboxed_const 0L ^^ set_ref_buf env ^^
      compile_unboxed_const 0L ^^ set_typtbl env ^^
      compile_unboxed_const 0L ^^ set_typtbl_end env ^^
      compile_unboxed_const 0L ^^ set_typtbl_size env ^^ (* also reset for symmetry, even if no pointer *)
      compile_unboxed_const 0L ^^ set_global_typtbl env ^^
      compile_unboxed_const 0L ^^ set_global_typtbl_end env ^^
      compile_unboxed_const 0L ^^ set_global_typtbl_size env (* also reset for symmetry, even if no pointer *)

    let get_value_quota env =
      G.i (GlobalGet (nr (E.get_global env "@@value_quota")))
    let set_value_quota env =
      G.i (GlobalSet (nr (E.get_global env "@@value_quota")))

    let get_value_numerator env =
      G.i (GlobalGet (nr (E.get_global env "@@value_numerator")))
    let set_value_numerator env =
      G.i (GlobalSet (nr (E.get_global env "@@value_numerator")))

    let get_value_denominator env =
      G.i (GlobalGet (nr (E.get_global env "@@value_denominator")))
    let set_value_denominator env =
      G.i (GlobalSet (nr (E.get_global env "@@value_denominator")))

    let get_value_bias env =
      G.i (GlobalGet (nr (E.get_global env "@@value_bias")))
    let set_value_bias env =
      G.i (GlobalSet (nr (E.get_global env "@@value_bias")))

    let reset_value_limit env get_blob get_rel_buf_opt =
      get_rel_buf_opt ^^
      E.if0
      begin (* Candid deserialization *)
        (* Set instruction limit *)
        let (set_product, get_product) = new_local env "product" in
        let (set_len, get_len) = new_local env "len" in
        get_blob ^^
        Blob.len env ^^
        set_len ^^

        get_len ^^
        get_value_numerator env ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^
        get_value_denominator env ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.DivU)) ^^
        set_product ^^

        get_value_numerator env ^^ (* check overflow for non-zero numerator *)
        (E.if0 begin
          (* Saturate multiplication `len * idl_value_numerator` on overflow.
             Ignore `idl_value_denomminator` on overflow. *)
          compile_unboxed_const (-1L) ^^ (* u64::MAX *)
          get_value_numerator env ^^ (* non-zero! *)
          G.i (Binary (Wasm_exts.Values.I64 I64Op.DivU)) ^^
          get_len ^^
          compile_comparison I64Op.LtU ^^
          (E.if0 begin
            compile_unboxed_const (-1L) ^^ (* u64::MAX *)
            set_product
          end
            G.nop)
        end
          G.nop) ^^

        get_product ^^
        get_value_bias env ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
        set_value_quota env ^^

        (* Saturate value_quota on overflow *)
        get_value_quota env ^^
        get_product ^^
        compile_comparison I64Op.LtU ^^
        E.if0 begin
          compile_unboxed_const (-1L) ^^ (* u64::MAX *)
          set_value_quota env
        end
          G.nop
      end
      begin (* Extended candid/ Destabilization *)
        G.nop
      end

    let get_type_scaler env =
      G.i (GlobalGet (nr (E.get_global env "@@type_scaler")))
    let set_type_scaler env =
      G.i (GlobalSet (nr (E.get_global env "@@type_scaler")))

    let get_type_bias env =
      G.i (GlobalGet (nr (E.get_global env "@@type_bias")))
    let set_type_bias env =
      G.i (GlobalSet (nr (E.get_global env "@@type_bias")))

    let define_idl_limit_check env =
      Func.define_built_in env "idl_limit_check"
        [("decrement", I32Type); ("count", I64Type)] [] (fun env ->
        get_rel_buf_opt env ^^
        E.if0 begin (* Candid deserialization *)
          get_value_quota env ^^
          G.i (LocalGet (nr 1l)) ^^ (* Count of values *)
          compile_comparison I64Op.LtU ^^
          E.then_trap_with env "IDL error: exceeded value limit" ^^
          (* if (decrement) quota -= count *)
          G.i (LocalGet (nr 0l)) ^^
          Bool.from_rts_int32 ^^
          E.if0 begin
             get_value_quota env ^^
             G.i (LocalGet (nr 1l)) ^^
             G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
             set_value_quota env
           end
             G.nop
        end begin (* Extended Candid/Destabilization *)
          G.nop
        end)

    let idl_limit_check env =
      G.i (Call (nr (E.built_in env "idl_limit_check")))

  end

  open Typ_hash

  let sort_by_hash fs =
    List.sort
      (fun (h1,_) (h2,_) -> Lib.Uint32.compare h1 h2)
      (List.map (fun f -> (Idllib.Escape.unescape_hash f.Type.lab, f)) fs)

  (* The IDL serialization prefaces the data with a type description.
     We can statically create the type description in Ocaml code,
     store it in the program, and just copy it to the beginning of the message.

     At some point this can be factored into a function from Motoko type to IDL,
     type and a function like this for IDL types. But due to recursion handling
     it is easier to start like this.
  *)

  module TM = Map.Make (Type.Ord)

  type mode =
    | Candid
    | Persistence

  let to_idl_prim mode = let open Type in function
    | Prim Null | Tup [] -> Some 1l
    | Prim Bool -> Some 2l
    | Prim Nat -> Some 3l
    | Prim Int -> Some 4l
    | Prim Nat8 -> Some 5l
    | Prim Nat16 -> Some 6l
    | Prim (Nat32|Char) -> Some 7l
    | Prim Nat64 -> Some 8l
    | Prim Int8 -> Some 9l
    | Prim Int16 -> Some 10l
    | Prim Int32 -> Some 11l
    | Prim Int64 -> Some 12l
    | Prim Float -> Some 14l
    | Prim Text -> Some 15l
    (* NB: Prim Blob does not map to a primitive IDL type *)
    | Any -> Some 16l
    | Non -> Some 17l
    | Prim Principal -> Some 24l
    | Prim Region -> Some 128l
    (* only used for memory compatibility checks *)
    | Prim Blob ->
      (match mode with
      | Candid -> None
      | Persistence -> Some 129l)
    | _ -> None

  (* some constants, also see rts/idl.c *)
  let idl_opt       = -18l
  let idl_vec       = -19l
  let idl_record    = -20l
  let idl_variant   = -21l
  let idl_func      = -22l
  let idl_service   = -23l
  let idl_alias     = 1l (* see Note [mutable stable values] *)

  (* only used for memory compatibility checks *)
  let idl_tuple     = -130l
  let idl_weak     = -131l (* UNUSED FOR NOW, might need eventually *)


  (* TODO: use record *)
  let type_desc env mode ts :
     string * int list * int32 list  (* type_desc, (relative offsets), indices of ts *)
    =
    let open Type in

    (* Type traversal *)
    (* We do a first traversal to find out the indices of non-primitive types *)
    let (typs, idx) =
      let typs = ref Table.empty in
      let idx = ref TM.empty in
      let rec go t =
        let t = Type.normalize t in
        if to_idl_prim mode t <> None then () else
        if TM.mem t !idx then () else begin
          let (i, tbl) = Table.add !typs t in
          typs := tbl;
          idx := TM.add t (Int32.of_int i) !idx;
          match t with
          | Tup ts -> List.iter go ts
          | Obj (_, fs) ->
            List.iter (fun f -> go f.typ) fs
          | Array (Mut t) -> go (Array t)
          | Array t -> go t
          | Opt t -> go t
          | Variant vs -> List.iter (fun f -> go f.typ) vs
          | Func (s, c, tbs, ts1, ts2) ->
            List.iter go ts1; List.iter go ts2
          | Prim Blob -> ()
          | Weak t -> go t
          | Mut t -> go t
          | _ ->
            Printf.eprintf "type_desc: unexpected type %s\n" (string_of_typ t);
            assert false
        end
      in
      List.iter go ts;
      (Table.to_list !typs, !idx)
    in

    (* buffer utilities *)
    let buf = Buffer.create 16 in

    let add_u8 i =
      Buffer.add_char buf (Char.chr (i land 0xff)) in

    let rec add_leb128_32 (i : Lib.Uint32.t) =
      let open Lib.Uint32 in
      let b = logand i (of_int32 0x7fl) in
      if of_int32 0l <= i && i < of_int32 128l
      then add_u8 (to_int b)
      else begin
        add_u8 (to_int (logor b (of_int32 0x80l)));
        add_leb128_32 (shift_right_logical i 7)
      end in

    let add_leb128 i =
      assert (i >= 0);
      add_leb128_32 (Lib.Uint32.of_int i) in

    let rec add_sleb128 (i : int32) =
      let open Int32 in
      let b = logand i 0x7fl in
      if -64l <= i && i < 64l
      then add_u8 (to_int b)
      else begin
        add_u8 (to_int (logor b 0x80l));
        add_sleb128 (shift_right i 7)
      end in

    (* Actual binary data *)

    let add_idx t =
      let t = Type.normalize t in
      match to_idl_prim mode t with
      | Some i -> add_sleb128 (Int32.neg i)
      | None -> add_sleb128 (TM.find (normalize t) idx) in

    let idx t =
      let t = Type.normalize t in
      match to_idl_prim mode t with
      | Some i -> Int32.neg i
      | None -> TM.find (normalize t) idx in

    let rec add_typ t =
      match t with
      | Non -> assert false
      | Prim Blob ->
        assert (mode = Candid);
        add_typ Type.(Array (Prim Nat8))
      | Prim Region ->
        add_sleb128 idl_alias; add_idx t
      | Prim _ -> assert false
      | Tup ts ->
        add_sleb128 (match mode with
        | Candid -> idl_record
        | Persistence -> idl_tuple);
        add_leb128 (List.length ts);
        List.iteri (fun i t ->
          add_leb128 i;
          add_idx t;
        ) ts
      | Obj ((Object | Memory), fs) ->
        add_sleb128 idl_record;
        add_leb128 (List.length fs);
        List.iter (fun (h, f) ->
          add_leb128_32 h;
          add_idx f.typ
        ) (sort_by_hash fs)
      | Array (Mut t) ->
        add_sleb128 idl_alias; add_idx (Array t)
      | Array t ->
        add_sleb128 idl_vec; add_idx t
      | Opt t ->
        add_sleb128 idl_opt; add_idx t
      | Variant vs ->
        add_sleb128 idl_variant;
        add_leb128 (List.length vs);
        List.iter (fun (h, f) ->
          add_leb128_32 h;
          add_idx f.typ
        ) (sort_by_hash vs)
      | Func (s, c, tbs, ts1, ts2) ->
        assert (Type.is_shared_sort s);
        add_sleb128 idl_func;
        add_leb128 (List.length ts1);
        List.iter add_idx ts1;
        add_leb128 (List.length ts2);
        List.iter add_idx ts2;
        begin match s, c with
          | _, Returns ->
            add_leb128 1; add_u8 2; (* oneway *)
          | Shared Write, _ ->
            add_leb128 0; (* no annotation *)
          | Shared Query, _ ->
            add_leb128 1; add_u8 1; (* query *)
          | Shared Composite, _ ->
            add_leb128 1; add_u8 3; (* composite *)
          | _ -> assert false
        end
      | Obj (Actor, fs) ->
        add_sleb128 idl_service;
        add_leb128 (List.length fs);
        List.iter (fun f ->
          add_leb128 (String.length f.lab);
          Buffer.add_string buf f.lab;
          add_idx f.typ
        ) fs
      | Mut t ->
         add_sleb128 idl_alias; add_idx t
      | Weak t ->
         add_sleb128 idl_weak; add_idx t
      | _ -> assert false in

    Buffer.add_string buf "DIDL";
    add_leb128 (List.length typs);
    let offsets = List.map (fun typ ->
      let offset = Buffer.length buf in
      add_typ typ;
      offset)
      typs
    in
    add_leb128 (List.length ts);
    List.iter add_idx ts;
    (Buffer.contents buf,
     offsets,
     List.map idx ts)

  (* See Note [Candid subtype checks] *)
  let reserve_global_type_descriptor (env : E.t) =
    let candid_data_segment = E.add_data_segment env "" in
    let type_offsets_segment = E.add_data_segment env "" in
    let idl_types_segment = E.add_data_segment env "" in
    E.(env.global_type_descriptor := Some {
      candid_data_segment;
      type_offsets_segment;
      idl_types_segment;
    })

  let create_global_type_descriptor (env : E.t) (set_candid_data_length, set_type_offsets_length, set_idl_types_length) =
    let descriptor = get_global_type_descriptor env in
    let candid_data, type_offsets, idl_types = type_desc env Candid (E.get_typtbl_typs env) in
    let candid_data_binary = [StaticBytes.Bytes candid_data] in
    let candid_data_length = E.replace_data_segment env E.(descriptor.candid_data_segment) candid_data_binary in
    set_candid_data_length candid_data_length;
    let type_offsets_binary = [StaticBytes.i64s (List.map Int64.of_int type_offsets)] in
    let type_offsets_length = E.replace_data_segment env E.(descriptor.type_offsets_segment) type_offsets_binary in
    set_type_offsets_length type_offsets_length;
    let idl_types_64 = List.map Wasm.I64_convert.extend_i32_u idl_types in
    let idl_types_binary = [StaticBytes.i64s idl_types_64] in
    let idl_types_length = E.replace_data_segment env E.(descriptor.idl_types_segment) idl_types_binary in
    set_idl_types_length idl_types_length

  (* Returns data (in bytes) and reference buffer size (in entries) needed *)
  let rec buffer_size env t =
    let open Type in
    let t = Type.normalize t in
    let name = "@buffer_size<" ^ typ_hash t ^ ">" in
    Func.share_code1 Func.Always env name ("x", I64Type) [I64Type; I64Type]
    (fun env get_x ->

      (* Some combinators for writing values *)
      let (set_data_size, get_data_size) = new_local env "data_size" in
      let (set_ref_size, get_ref_size) = new_local env "ref_size" in
      compile_unboxed_const 0L ^^ set_data_size ^^
      compile_unboxed_const 0L ^^ set_ref_size ^^

      let inc_data_size code =
        get_data_size ^^
        code ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
        set_data_size
      in

      let size_word env code =
        let (set_word, get_word) = new_local env "word" in
        code ^^ set_word ^^
        inc_data_size (I32Leb.compile_leb128_size get_word)
      in

      let size env t =
        let (set_inc, get_inc) = new_local env "inc" in
        buffer_size env t ^^
        get_ref_size ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^ set_ref_size ^^
        set_inc ^^ inc_data_size get_inc
      in

      (* the incremental GC leaves array slice information in tag,
         the slice information can be removed and the tag reset to array
         as the GC can resume marking from the array beginning *)
      let clear_array_slicing =
        let (set_temp, get_temp) = new_local env "temp" in
        set_temp ^^
        get_temp ^^ compile_unboxed_const Tagged.(int_of_tag StableSeen) ^^
        compile_comparison I64Op.Ne ^^
        get_temp ^^ compile_unboxed_const Tagged.(int_of_tag CoercionFailure) ^^
        compile_comparison I64Op.Ne ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
        get_temp ^^ compile_unboxed_const Tagged.(int_of_tag ArraySliceMinimum) ^^
        compile_comparison I64Op.GeU ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
        E.if1 I64Type begin
          (compile_unboxed_const Tagged.(int_of_tag (Array M)))
        end begin
          get_temp
        end
      in

      let size_alias size_thing =
        (* see Note [mutable stable values] *)
        let (set_tag, get_tag) = new_local env "tag" in
        get_x ^^ Tagged.load_tag env ^^ clear_array_slicing ^^ set_tag ^^
        (* For now, trap on WeakRef *)
        get_tag ^^ compile_eq_const Tagged.(int_of_tag WeakRef) ^^
        E.then_trap_with env "object_size/Mut: Unexpected tag WeakRef" ^^
        (* Sanity check *)
        get_tag ^^ compile_eq_const Tagged.(int_of_tag WeakRef) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        get_tag ^^ compile_eq_const Tagged.(int_of_tag StableSeen) ^^
        get_tag ^^ compile_eq_const Tagged.(int_of_tag MutBox) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        get_tag ^^ compile_eq_const Tagged.(int_of_tag (Array M)) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        get_tag ^^ compile_eq_const Tagged.(int_of_tag Region) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        E.else_trap_with env "object_size/Mut: Unexpected tag." ^^
        (* Check if we have seen this before *)
        get_tag ^^ compile_eq_const Tagged.(int_of_tag StableSeen) ^^
        E.if0 begin
          (* Seen before *)
          (* One byte marker, one word offset *)
          (* TODO: Support 64-bit pointer in serialization *)
          inc_data_size (compile_unboxed_const 5L)
        end begin
          (* Not yet seen *)
          (* One byte marker, two words scratch space *)
          inc_data_size (compile_unboxed_const 9L) ^^
          (* Mark it as seen *)
          get_x ^^ Tagged.(store_tag env StableSeen) ^^
          (* and descend *)
          size_thing ()
        end
      in

      (* Now the actual type-dependent code *)
      begin match t with
      | Prim Nat -> inc_data_size (get_x ^^ BigNum.compile_data_size_unsigned env)
      | Prim Int -> inc_data_size (get_x ^^ BigNum.compile_data_size_signed env)
      | Prim (Int8|Nat8) -> inc_data_size compile_unboxed_one
      | Prim (Int16|Nat16) -> inc_data_size (compile_unboxed_const 2L)
      | Prim (Int32|Nat32|Char) -> inc_data_size (compile_unboxed_const 4L)
      | Prim (Int64|Nat64|Float) -> inc_data_size (compile_unboxed_const 8L)
      | Prim Bool -> inc_data_size compile_unboxed_one
      | Prim Null -> G.nop
      | Any -> G.nop
      | Tup [] -> G.nop (* e(()) = null *)
      | Tup ts ->
        G.concat_mapi (fun i t ->
          get_x ^^ Tuple.load_n env (Int64.of_int i) ^^
          size env t
          ) ts
      | Obj ((Object | Memory), fs) ->
        G.concat_map (fun (_h, f) ->
          get_x ^^ Object.load_idx_raw env f.Type.lab ^^
          size env f.typ
          ) (sort_by_hash fs)
      | Array (Mut t) ->
        size_alias (fun () -> get_x ^^ size env (Array t))
      | Array t ->
        size_word env (get_x ^^ Arr.len env) ^^
        get_x ^^ Arr.len env ^^
        from_0_to_n env (fun get_i ->
          get_x ^^ get_i ^^ Arr.unsafe_idx env ^^ load_ptr ^^
          size env t
        )
      | Prim Blob ->
        let (set_len, get_len) = new_local env "len" in
        get_x ^^ Blob.len env ^^ set_len ^^
        size_word env get_len ^^
        inc_data_size get_len
      | Prim Text ->
        let (set_len, get_len) = new_local env "len" in
        get_x ^^ Text.size env ^^ set_len ^^
        size_word env get_len ^^
        inc_data_size get_len
      | Opt t ->
        inc_data_size compile_unboxed_one ^^ (* one byte tag *)
        get_x ^^ Opt.is_some env ^^
        E.if0 (get_x ^^ Opt.project env ^^ size env t) G.nop
      | Variant vs ->
        List.fold_right (fun (i, {lab = l; typ = t; _}) continue ->
            get_x ^^
            Variant.test_is env l ^^
            E.if0
              ( size_word env (compile_unboxed_const (Int64.of_int i)) ^^
                get_x ^^ Variant.project env ^^ size env t
              ) continue
          )
          ( List.mapi (fun i (_h, f) -> (i,f)) (sort_by_hash vs) )
          ( E.trap_with env "buffer_size: unexpected variant" )
      | Func _ ->
        inc_data_size compile_unboxed_one ^^ (* one byte tag *)
        get_x ^^ Arr.load_field env 0L ^^ size env (Obj (Actor, [])) ^^
        get_x ^^ Arr.load_field env 1L ^^ size env (Prim Text)
      | Obj (Actor, _) | Prim Principal ->
        inc_data_size compile_unboxed_one ^^ (* one byte tag *)
        get_x ^^ size env blob
      | Non ->
        E.trap_with env "buffer_size called on value of type None"
      | Prim Region ->
         size_alias (fun () ->
          inc_data_size (compile_unboxed_const 12L) ^^ (* |id| + |page_count| = 8 + 4 *)
          get_x ^^ Region.vec_pages env ^^ size env blob)
      | Mut t ->
         size_alias (fun () -> get_x ^^ MutBox.load_field env ^^ size env t)
      | Weak t ->
         E.trap_with env "buffer_size: Weak" ^^
         size_alias (fun () -> get_x ^^ WeakRef.load_field env ^^ size env t)
      | _ -> todo "buffer_size" (Arrange_ir.typ t) G.nop
      end ^^
      (* Check 32-bit overflow of buffer_size *)
      (* TODO: Support 64-bit buffer *)
      get_data_size ^^
      compile_shrU_const 32L ^^
      compile_test I64Op.Eqz ^^
      E.else_trap_with env "buffer_size overflow" ^^
      get_data_size ^^
      get_ref_size
    )

  (* TODO: Change serialization version because format changes with 64-bit *)
  (* Copies x to the data_buffer, storing references after ref_count entries in ref_base *)
  let rec serialize_go env t =
    let open Type in
    let t = Type.normalize t in
    let name = Strm.name_for "serialize_go" [t] in
    Func.share_code3 Func.Always env name (("x", I64Type), ("data_buffer", I64Type), ("ref_buffer", I64Type)) [I64Type; I64Type]
    (fun env get_x get_data_buf get_ref_buf ->
      let set_ref_buf = G.setter_for get_ref_buf in

      (* Some combinators for writing values *)
      let open Strm in

      let write env t =
        get_data_buf ^^
        get_ref_buf ^^
        serialize_go env t ^^
        set_ref_buf ^^
        checkpoint env get_data_buf
      in

      let write_alias write_thing =
        (* see Note [mutable stable values] *)
        (* Check heap tag *)
        let (set_tag, get_tag) = new_local env "tag" in
        get_x ^^ Tagged.load_tag env ^^ set_tag ^^
        get_tag ^^ compile_eq_const Tagged.(int_of_tag StableSeen) ^^
        E.if0
        begin
          (* This is the real data *)
          write_byte env get_data_buf (compile_unboxed_const 0L) ^^
          (* Remember the current offset in the tag word *)
          get_x ^^ Tagged.load_forwarding_pointer env ^^ Strm.absolute_offset env get_data_buf ^^
          Tagged.store_field env Tagged.tag_field ^^
          (* Leave space in the output buffer for the decoder's bookkeeping *)
          (* For compatibility, this is still in 32-bit format, but not useful in 64-bit *)
          write_word_32 env get_data_buf (compile_unboxed_const 0L) ^^
          write_word_32 env get_data_buf (compile_unboxed_const 0L) ^^
          (* Now the data, following the object field mutbox indirection *)
          write_thing ()
        end
        begin
          (* This is a reference *)
          write_byte env get_data_buf compile_unboxed_one ^^
          (* Sanity Checks *)
          get_tag ^^ compile_eq_const Tagged.(int_of_tag WeakRef) ^^
          E.then_trap_with env "unvisited mutable data in serialize_go (WeakRef)" ^^
          get_tag ^^ compile_eq_const Tagged.(int_of_tag MutBox) ^^
          E.then_trap_with env "unvisited mutable data in serialize_go (MutBox)" ^^
          get_tag ^^ compile_eq_const Tagged.(int_of_tag (Array M)) ^^
          E.then_trap_with env "unvisited mutable data in serialize_go (Array)" ^^
          get_tag ^^ compile_eq_const Tagged.(int_of_tag Region) ^^
          E.then_trap_with env "unvisited mutable data in serialize_go (Region)" ^^
          (* Second time we see this *)
          (* Calculate relative offset *)
          let set_offset, get_offset = new_local env "offset" in
          get_tag ^^ Strm.absolute_offset env get_data_buf ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)) ^^
          set_offset ^^
          (* A sanity check *)
          get_offset ^^ compile_unboxed_const 0L ^^
          compile_comparison I64Op.LtS ^^
          E.else_trap_with env "Odd offset" ^^
          (* TODO: Support serialization beyond 32-bit *)
          get_offset ^^ compile_unboxed_const 0xffff_ffff_0000_0000L ^^
          compile_comparison I64Op.GeS ^^
          E.else_trap_with env "64-bit offsets not yet supported during serialization" ^^
          (* Write the offset to the output buffer *)
          write_word_32 env get_data_buf get_offset
        end
      in

      (* Now the actual serialization *)

      begin match t with
      | Prim Nat ->
        write_bignum_leb env get_data_buf get_x
      | Prim Int ->
        write_bignum_sleb env get_data_buf get_x
      | Prim Float ->
        reserve env get_data_buf 8L ^^
        get_x ^^ Float.unbox env ^^
        G.i (Store {ty = F64Type; align = 0; offset = 0L; sz = None})
      | Prim ((Int64|Nat64) as pty) ->
        reserve env get_data_buf 8L ^^
        get_x ^^ BoxedWord64.unbox env pty ^^
        G.i (Store {ty = I64Type; align = 0; offset = 0L; sz = None})
      | Prim ((Int32|Nat32) as ty) ->
        write_word_32 env get_data_buf (get_x ^^ TaggedSmallWord.lsb_adjust ty)
      | Prim Char ->
        write_word_32 env get_data_buf (get_x ^^ TaggedSmallWord.lsb_adjust_codepoint env)
      | Prim ((Int16|Nat16) as ty) ->
        reserve env get_data_buf 2L ^^
        get_x ^^ TaggedSmallWord.lsb_adjust ty ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
        G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = Some Wasm_exts.Types.Pack16})
      | Prim ((Int8|Nat8) as ty) ->
        write_byte env get_data_buf (get_x ^^ TaggedSmallWord.lsb_adjust ty)
      | Prim Bool ->
        write_byte env get_data_buf get_x
      | Tup [] -> (* e(()) = null *)
        G.nop
      | Tup ts ->
        G.concat_mapi (fun i t ->
          get_x ^^ Tuple.load_n env (Int64.of_int i) ^^
          write env t
        ) ts
      | Obj ((Object | Memory), fs) ->
        G.concat_map (fun (_h, f) ->
          get_x ^^ Object.load_idx_raw env f.Type.lab ^^
          write env f.typ
        ) (sort_by_hash fs)
      | Array (Mut t) ->
        write_alias (fun () -> get_x ^^ write env (Array t))
      | Prim Region ->
        write_alias (fun () ->
          reserve env get_data_buf 8L ^^
          get_x ^^ Region.id env ^^
          G.i (Store {ty = I64Type; align = 0; offset = 0L; sz = None}) ^^
          write_word_32 env get_data_buf (get_x ^^ Region.page_count env) ^^
          write_blob env get_data_buf (get_x ^^ Region.vec_pages env)
        )
      | Array t ->
        write_word_leb env get_data_buf (get_x ^^ Arr.len env) ^^
        get_x ^^ Arr.len env ^^
        from_0_to_n env (fun get_i ->
          get_x ^^ get_i ^^ Arr.unsafe_idx env ^^ load_ptr ^^
          write env t
        )
      | Prim Null -> G.nop
      | Any -> G.nop
      | Opt t ->
        get_x ^^
        Opt.is_some env ^^
        E.if0
          (write_byte env get_data_buf compile_unboxed_one ^^ get_x ^^ Opt.project env ^^ write env t)
          (write_byte env get_data_buf (compile_unboxed_const 0L))
      | Variant vs ->
        List.fold_right (fun (i, {lab = l; typ = t; _}) continue ->
            get_x ^^
            Variant.test_is env l ^^
            E.if0
              ( write_word_leb env get_data_buf (compile_unboxed_const (Int64.of_int i)) ^^
                get_x ^^ Variant.project env ^^ write env t)
              continue
          )
          ( List.mapi (fun i (_h, f) -> (i,f)) (sort_by_hash vs) )
          ( E.trap_with env "serialize_go: unexpected variant" )
      | Prim Blob ->
        write_blob env get_data_buf get_x
      | Prim Text ->
        write_text env get_data_buf get_x
      | Func _ ->
        write_byte env get_data_buf compile_unboxed_one ^^
        get_x ^^ Arr.load_field env 0L ^^ write env (Obj (Actor, [])) ^^
        get_x ^^ Arr.load_field env 1L ^^ write env (Prim Text)
      | Obj (Actor, _) | Prim Principal ->
        write_byte env get_data_buf compile_unboxed_one ^^
        get_x ^^ write env blob
      | Non ->
        E.trap_with env "serializing value of type None"
      | Mut t ->
        write_alias (fun () ->
          get_x ^^ MutBox.load_field env ^^ write env t
          )
      | Weak t ->
        E.trap_with env "serialize_go: Weak" ^^
        write_alias (fun () ->
          get_x ^^ WeakRef.load_field env ^^ write env t
        )
      | _ -> todo "serialize" (Arrange_ir.typ t) G.nop
      end ^^
      get_data_buf ^^
      get_ref_buf
    )

  (* This value is returned by deserialize_go if deserialization fails in a way
     that should be recoverable by opt parsing.
     It is an (invalid) sentinel pointer value (in skewed format) and can be used for pointer comparison.
     It will be never placed on the heap and must not be dereferenced.
     If unskewed, it refers to the unallocated last Wasm memory page.
  *)
  let coercion_error_value env = 0xffff_ffff_ffff_fffdL

  (* See Note [Candid subtype checks] *)
  let with_rel_buf_opt env extended get_typtbl_size1 get_typtbl_size2 f =
    if extended then
      f (compile_unboxed_const 0L)
    else
      get_typtbl_size1 ^^ get_typtbl_size2 ^^
      E.call_import env "rts" "idl_sub_buf_words" ^^
      Stack.dynamic_with_words env "rel_buf" (fun get_ptr ->
        get_ptr ^^ get_typtbl_size1 ^^ get_typtbl_size2 ^^
        E.call_import env "rts" "idl_sub_buf_init" ^^
        f get_ptr)

  (* See Note [Candid subtype checks] *)
  let idl_sub env t2 =
    let idx = Wasm.I64_convert.extend_i32_u (E.add_typtbl_typ env t2) in
    get_global_idl_types env ^^
    Blob.payload_ptr_unskewed env ^^
    G.i (Load {ty = I64Type; align = 0; offset = Int64.mul idx candid_type_offset_size (*!*); sz = None}) ^^
    Func.share_code2 Func.Always env ("idl_sub")
      (("idltyp1", I64Type),
       ("idltyp2", I64Type)
      )
      [I64Type]
      (fun env get_idltyp1 get_idltyp2 ->
        Registers.get_rel_buf_opt env ^^
        E.else_trap_with env "null rel_buf" ^^
        Registers.get_rel_buf_opt env ^^
        Registers.get_typtbl env ^^
        Registers.get_global_typtbl env ^^
        Registers.get_typtbl_end env ^^
        Registers.get_global_typtbl_end env ^^
        Registers.get_typtbl_size env ^^
        Registers.get_global_typtbl_size env ^^
        get_idltyp1 ^^
        G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
        get_idltyp2 ^^
        G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
        E.call_import env "rts" "idl_sub" ^^
        G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
        )

  (* The main deserialization function, generated once per type hash.

     We use a combination of RTS stack locals and registers (Wasm globals) for
     recursive parameter passing to avoid exhausting the Wasm stack, which is instead
     used solely for return values and (implicit) return addresses.

     Its RTS stack parameters are (c.f. module Stack):

       * idltyp:      The idl type (prim type or table index) to decode now
       * depth:       Recursion counter; reset when we make progres on the value
       * can_recover: Whether coercion errors are recoverable, see coercion_failed below

     Its register parameters are (c.f. Registers):
       * rel_buf_opt: The optional subtype check memoization table
          (non-null for untrusted Candid but null for trusted de-stablization (see `with_rel_buf_opt`).)
       * data_buffer: The current position of the input data buffer
       * ref_buffer:  The current position of the input references buffer
       * typtbl:      The type table, as returned by parse_idl_header
       * typtbl_size: The size of the type table, used to limit recursion

     It returns the value of type t (vanilla representation) or coercion_error_value,
     It advances the data_buffer past the decoded value (even if it returns coercion_error_value!)

  *)

  (* symbolic names for arguments passed on RTS stack *)
  module StackArgs = struct
    let idltyp = 0L
    let depth = 1L
    let can_recover = 2L
  end

  let rec deserialize_go env t =
    let open Type in
    let t = Type.normalize t in
    let name = "@deserialize_go<" ^ typ_hash t ^ ">" in
    Func.share_code0 Func.Always env name
      [I64Type]
      (fun env  ->
      let get_idltyp = Stack.get_local env StackArgs.idltyp in
      let get_depth = Stack.get_local env StackArgs.depth in
      let get_can_recover = Stack.get_local env StackArgs.can_recover in
      let get_rel_buf_opt = Registers.get_rel_buf_opt env in
      let get_data_buf = Registers.get_data_buf env in
      let _get_ref_buf = Registers.get_ref_buf env in
      let get_typtbl = Registers.get_typtbl env in
      let _get_typtbl_end = Registers.get_typtbl_end env in
      let get_typtbl_size = Registers.get_typtbl_size env in

      (* Decrement and check idl quota *)
      Bool.lit_rts_int32 true ^^
      compile_unboxed_one ^^
      Registers.idl_limit_check env ^^

      (* Check recursion depth (protects against empty record etc.) *)
      (* Factor 2 because at each step, the expected type could go through one
         level of opt that is not present in the value type
      *)
      get_depth ^^
      get_typtbl_size ^^ compile_add_const 1L ^^ compile_mul_const 2L ^^
      compile_comparison I64Op.LeU ^^
      E.else_trap_with env ("IDL error: circular record read") ^^

      (* Remember data buffer position, to detect progress *)
      let (set_old_pos, get_old_pos) = new_local env "old_pos" in
      ReadBuf.get_ptr get_data_buf ^^ set_old_pos ^^

      let go' can_recover env t =
        (* assumes idltyp on stack *)
        Stack.with_frame env "frame_ptr" 3L (fun () ->
          Stack.set_local env StackArgs.idltyp ^^
          (* set up frame arguments *)
          ( (* Reset depth counter if we made progress *)
            ReadBuf.get_ptr get_data_buf ^^ get_old_pos ^^
            compile_comparison I64Op.Eq ^^
            E.if1 I64Type
              (Stack.get_prev_local env 1L ^^ compile_add_const 1L)
              (compile_unboxed_const 0L)
            ) ^^
          Stack.set_local env StackArgs.depth ^^
          (if can_recover
             then compile_unboxed_one
             else Stack.get_prev_local env 2L) ^^
          Stack.set_local env StackArgs.can_recover ^^
          deserialize_go env t)
      in

      let go = go' false in
      let go_can_recover = go' true in

      let skip get_typ =
        get_data_buf ^^ get_typtbl ^^ get_typ ^^  G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^ compile_const_32 0l ^^
        E.call_import env "rts" "skip_any"
      in

      (* This flag is set to return a coercion error at the very end
         We cannot use (G.i Return) for early exit, or we’d leak stack space,
         as Stack.with_words is used to allocate scratch space.
      *)
      let (set_failed, get_failed) = new_local env "failed" in
      let set_failure = compile_unboxed_one ^^ set_failed in
      let when_failed f = get_failed ^^ E.if0 f G.nop in

      (* This looks at a value and if it is coercion_error_value, sets the failure flag.
         This propagates the error out of arrays, records, etc.
       *)
      let remember_failure get_val =
          get_val ^^ compile_eq_const (coercion_error_value env) ^^
          E.if0 set_failure G.nop
      in

      (* This sets the failure flag and puts coercion_error_value on the stack *)
      let coercion_failed msg =
        (* If we know that there is no backtracking `opt t` around, then just trap.
           This gives a better error message
        *)
        get_can_recover ^^ E.else_trap_with env msg ^^
        set_failure ^^ compile_unboxed_const (coercion_error_value env) in

      (* returns true if we are looking at primitive type with this id *)
      let check_prim_typ t =
        get_idltyp ^^
        compile_eq_const (Wasm.I64_convert.extend_i32_s (Int32.neg (Option.get (to_idl_prim Candid t))))
      in

      let with_prim_typ t f =
        check_prim_typ t ^^
        E.if1 I64Type f
          ( skip get_idltyp ^^
            coercion_failed ("IDL error: unexpected IDL type when parsing " ^ string_of_typ t)
          )
      in

      let read_byte_tagged = function
        | [code0; code1] ->
          ReadBuf.read_byte env get_data_buf ^^
          let (set_b, get_b) = new_local env "b" in
          set_b ^^
          get_b ^^
          compile_eq_const 0L ^^
          E.if1 I64Type
          begin code0
          end begin
            get_b ^^ compile_eq_const 1L ^^
            E.else_trap_with env "IDL error: byte tag not 0 or 1" ^^
            code1
          end
        | _ -> assert false; (* can be generalized later as needed *)
      in

      let read_blob () =
        let (set_len, get_len) = new_local env "len" in
        let (set_x, get_x) = new_local env "x" in
        ReadBuf.read_leb128 env get_data_buf ^^ set_len ^^

        Blob.alloc env Tagged.B get_len ^^ set_x ^^
        get_x ^^ Blob.payload_ptr_unskewed env ^^
        ReadBuf.read_blob env get_data_buf get_len ^^
        get_x
      in

      let read_principal sort () =
        let (set_len, get_len) = new_local env "len" in
        let (set_x, get_x) = new_local env "x" in
        ReadBuf.read_leb128 env get_data_buf ^^ set_len ^^

        (* at most 29 bytes, according to
           https://sdk.dfinity.org/docs/interface-spec/index.html#principal
        *)
        get_len ^^ compile_unboxed_const 29L ^^ compile_comparison I64Op.LeU ^^
        E.else_trap_with env "IDL error: principal too long" ^^

        Blob.alloc env sort get_len ^^ set_x ^^
        get_x ^^ Blob.payload_ptr_unskewed env ^^
        ReadBuf.read_blob env get_data_buf get_len ^^
        get_x
      in

      let read_text () =
        let (set_len, get_len) = new_local env "len" in
        ReadBuf.read_leb128 env get_data_buf ^^ set_len ^^
        let (set_ptr, get_ptr) = new_local env "x" in
        ReadBuf.get_ptr get_data_buf ^^ set_ptr ^^
        ReadBuf.advance get_data_buf get_len ^^
        (* validate *)
        get_ptr ^^ get_len ^^ E.call_import env "rts" "utf8_validate" ^^
        (* copy *)
        get_ptr ^^ get_len ^^ Text.of_ptr_size env
      in

      let read_actor_data () =
        read_byte_tagged
          [ E.trap_with env "IDL error: unexpected actor reference"
          ; read_principal Tagged.A ()
          ]
      in

      (* returns true if get_arg_typ is a composite type of this id *)
      let check_composite_typ get_arg_typ idl_tycon_id =
        get_arg_typ ^^
        compile_unboxed_const 0L ^^ compile_comparison I64Op.GeS ^^
        E.if1 I64Type
        begin
          ReadBuf.alloc env (fun get_typ_buf ->
            (* Update typ_buf *)
            ReadBuf.set_ptr get_typ_buf (
              get_typtbl ^^
              get_arg_typ ^^ compile_mul_const Heap.word_size ^^
              G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
              load_unskewed_ptr
            ) ^^
            ReadBuf.set_end get_typ_buf (ReadBuf.get_end get_data_buf) ^^
            (* read sleb128 *)
            ReadBuf.read_sleb128 env get_typ_buf ^^
            (* Check it is the expected value *)
            compile_eq_const (Wasm.I64_convert.extend_i32_s idl_tycon_id)
          )
        end
        (compile_unboxed_const 0L)
      in


      (* checks that arg_typ is positive, looks it up in the table,
         creates a fresh typ_buf pointing into the type description,
         reads the type constructor index and traps or fails if it is the wrong one.
         and passes the typ_buf to a subcomputation to read the type arguments *)
      let with_composite_arg_typ get_arg_typ idl_tycon_id f =
        (* make sure index is not negative *)
        get_arg_typ ^^
        compile_unboxed_const 0L ^^ compile_comparison I64Op.GeS ^^
        E.if1 I64Type
        begin
          ReadBuf.alloc env (fun get_typ_buf ->
            (* Update typ_buf *)
            ReadBuf.set_ptr get_typ_buf (
              get_typtbl ^^
              get_arg_typ ^^ compile_mul_const Heap.word_size ^^
              G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
              load_unskewed_ptr
            ) ^^
            ReadBuf.set_end get_typ_buf (ReadBuf.get_end get_data_buf) ^^
            (* read sleb128 *)
            ReadBuf.read_sleb128 env get_typ_buf ^^
            (* Check it is the expected type constructor *)
            compile_eq_const (Wasm.I64_convert.extend_i32_s idl_tycon_id) ^^
            E.if1 I64Type
            begin
              f get_typ_buf
            end
            begin
              skip get_arg_typ ^^
              coercion_failed ("IDL error: unexpected IDL type when parsing " ^ string_of_typ t)
            end
          )
        end
        begin
          skip get_arg_typ ^^
          coercion_failed ("IDL error: unexpected IDL type when parsing " ^ string_of_typ t)
        end
      in

      let with_alias_typ get_arg_typ =
        get_arg_typ ^^
        compile_unboxed_const 0L ^^ compile_comparison I64Op.GeS ^^
        E.if1 I64Type
        begin
            with_composite_arg_typ get_arg_typ idl_alias (ReadBuf.read_sleb128 env)
        end
        begin
          (* sanity check *)
          get_arg_typ ^^
          compile_eq_const (Wasm.I64_convert.extend_i32_s (Int32.neg (Option.get (to_idl_prim Candid (Prim Region))))) ^^
          E.else_trap_with env "IDL error: unexpecting primitive alias type" ^^
          get_arg_typ
        end
      in

      let with_composite_typ idl_tycon_id f =
        with_composite_arg_typ get_idltyp idl_tycon_id f
      in

      let with_record_typ f = with_composite_typ idl_record (fun get_typ_buf ->
        Stack.with_words env "get_n_ptr" 1L (fun get_n_ptr ->
          get_n_ptr ^^
          ReadBuf.read_leb128 env get_typ_buf ^^
          store_unskewed_ptr ^^
          f get_typ_buf get_n_ptr
        )
      ) in

      let with_blob_typ env f =
        with_composite_typ idl_vec (fun get_typ_buf ->
          ReadBuf.read_sleb128 env get_typ_buf ^^
          compile_eq_const (-5L) (* Nat8 *) ^^
          E.if1 I64Type
            f
            begin
              skip get_idltyp ^^
              coercion_failed "IDL error: blob not a vector of nat8"
            end
        )
      in

      let store_word32 =
        G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
        G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None}) in

      (* See comment on 64-bit destabilization on Note [mutable stable values] *)
      let pointer_compression_shift = 3L in (* log2(word_size), 3 unused lower bits with 64-bit alignment *)

      let write_compressed_pointer env =
        let (set_pointer, get_pointer) = new_local env "pointer" in
        compile_add_const ptr_unskew ^^
        compile_shrU_const pointer_compression_shift ^^
        compile_add_const ptr_skew ^^
        set_pointer ^^ get_pointer ^^
        compile_unboxed_const 0xffff_ffffL ^^
        compile_comparison I64Op.LeU ^^
        E.else_trap_with env "Pointer cannot be compressed to 32 bit" ^^
        get_pointer ^^
        store_word32
      in

      let read_compressed_pointer env get_buf =
        let (set_pointer, get_pointer) = new_local env "pointer" in
        ReadBuf.read_word32 env get_buf ^^
        set_pointer ^^ get_pointer ^^
        compile_eq_const 0L ^^
        E.if1 I64Type
          begin
            get_pointer
          end
          begin
            get_pointer ^^
            compile_add_const ptr_unskew ^^
            compile_shl_const pointer_compression_shift ^^
            compile_add_const ptr_skew
          end
      in

      let read_alias env t read_thing =
        (* see Note [mutable stable values] *)
        let (set_is_ref, get_is_ref) = new_local env "is_ref" in
        let (set_result, get_result) = new_local env "result" in
        let (set_cur, get_cur) = new_local env "cur" in
        let (set_memo, get_memo) = new_local env "memo" in

        let (set_arg_typ, get_arg_typ) = new_local env "arg_typ" in

        with_alias_typ get_idltyp ^^ set_arg_typ ^^

        (* Find out if it is a reference or not *)
        ReadBuf.read_byte env get_data_buf ^^ set_is_ref ^^

        (* If it is a reference, temporarily set the read buffer to that place *)
        get_is_ref ^^
        E.if0 begin
          let (set_offset, get_offset) = new_local env "offset" in
          ReadBuf.read_signed_word32 env get_data_buf ^^
          set_offset ^^
          (* A sanity check *)
          get_offset ^^ compile_unboxed_const 0L ^^
          compile_comparison I64Op.LtS ^^
          E.else_trap_with env "Odd offset" ^^

          ReadBuf.get_ptr get_data_buf ^^ set_cur ^^
          ReadBuf.advance get_data_buf (get_offset ^^ compile_add_const (-4L))
        end G.nop ^^

        (* Remember location of ptr *)
        ReadBuf.get_ptr get_data_buf ^^ set_memo ^^
        (* Did we decode this already? *)
        read_compressed_pointer env get_data_buf ^^
        set_result ^^
        get_result ^^ compile_eq_const 0L ^^
        E.if0 begin
          (* No, not yet decoded *)
          (* Skip over type hash field *)
          ReadBuf.read_word32 env get_data_buf ^^ compile_eq_const 0L ^^
          E.else_trap_with env "Odd: Type hash scratch space not empty" ^^

          (* Read the content *)
          read_thing get_arg_typ (fun get_thing ->
            (* This is called after allocation, but before descending
               We update the memo location here so that loops work
            *)
            get_thing ^^ set_result ^^
            get_memo ^^ get_result ^^ write_compressed_pointer env ^^
            get_memo ^^ compile_add_const 4L ^^ Blob.lit env Tagged.B (typ_hash t) ^^ write_compressed_pointer env
          )
          end begin
          (* Decoded before. Check type hash *)
          read_compressed_pointer env get_data_buf ^^ Blob.lit env Tagged.B (typ_hash t) ^^
          Blob.compare env (Some Operator.EqOp) ^^
          E.else_trap_with env ("Stable memory error: Aliased at wrong type, expected: " ^ typ_hash t)
        end ^^

        (* If this was a reference, reset read buffer *)
        get_is_ref ^^
        E.if0 (ReadBuf.set_ptr get_data_buf get_cur) G.nop ^^

        get_result
      in


      (* Now the actual deserialization *)
      begin match t with
      (* Primitive types *)
      | Prim Nat ->
        with_prim_typ t
        begin
          BigNum.compile_load_from_data_buf env get_data_buf false
        end
      | Prim Int ->
        (* Subtyping with nat *)
        check_prim_typ (Prim Nat) ^^
        E.if1 I64Type
          begin
            BigNum.compile_load_from_data_buf env get_data_buf false
          end
          begin
            with_prim_typ t
            begin
              BigNum.compile_load_from_data_buf env get_data_buf true
            end
          end
      | Prim Float ->
        with_prim_typ t
        begin
          ReadBuf.read_float64 env get_data_buf ^^
          Float.box env
        end
      | Prim ((Int64|Nat64) as pty) ->
        with_prim_typ t
        begin
          ReadBuf.read_word64 env get_data_buf ^^
          BoxedWord64.box env pty
        end
      | Prim ((Int32|Nat32) as ty) ->
        with_prim_typ t
        begin
          ReadBuf.read_word32 env get_data_buf ^^
          TaggedSmallWord.msb_adjust ty ^^
          TaggedSmallWord.tag env ty
        end
      | Prim Char ->
        with_prim_typ t
        begin
          ReadBuf.read_word32 env get_data_buf ^^
          TaggedSmallWord.check_and_msb_adjust_codepoint env ^^
          TaggedSmallWord.tag env Char
        end
      | Prim ((Int16|Nat16) as ty) ->
        with_prim_typ t
        begin
          ReadBuf.read_word16 env get_data_buf ^^
          TaggedSmallWord.msb_adjust ty ^^
          TaggedSmallWord.tag env ty
        end
      | Prim ((Int8|Nat8) as ty) ->
        with_prim_typ t
        begin
          ReadBuf.read_byte env get_data_buf ^^
          TaggedSmallWord.msb_adjust ty ^^
          TaggedSmallWord.tag env ty
        end
      | Prim Bool ->
        with_prim_typ t
        begin
          read_byte_tagged
            [ Bool.lit false
            ; Bool.lit true
            ]
        end
      | Prim Null ->
        with_prim_typ t (Opt.null_lit env)
      | Any ->
        skip get_idltyp ^^
        (* Any vanilla value works here *)
        Opt.null_lit env
      | Prim Blob ->
        with_blob_typ env (
          let (set_blob, get_blob) = new_local env "blob" in
          read_blob () ^^ set_blob ^^  (* Read blob and save it *)
          compile_unboxed_zero ^^      (* Put closure on stack *)
          get_blob ^^                  (* Put blob on stack *)
          Internals.dedup env          (* Call dedup *)
        )
      | Prim Principal ->
        with_prim_typ t
        begin
          read_byte_tagged
            [ E.trap_with env "IDL error: unexpected principal reference"
            ; read_principal Tagged.P ()
            ]
        end
      | Prim Text ->
        with_prim_typ t (read_text ())
      | Tup [] -> (* e(()) = null *)
        with_prim_typ t (Tuple.from_stack env 0)
      (* Composite types *)
      | Tup ts ->
        with_record_typ (fun get_typ_buf get_n_ptr ->
          let (set_val, get_val) = new_local env "val" in

          G.concat_mapi (fun i t ->
            (* skip all possible intermediate extra fields *)
            get_typ_buf ^^ get_data_buf ^^ get_typtbl ^^ compile_const_32 (Int32.of_int i) ^^ get_n_ptr ^^
            E.call_import env "rts" "find_field" ^^
            Bool.from_rts_int32 ^^
            E.if1 I64Type
              begin
                ReadBuf.read_sleb128 env get_typ_buf ^^
                go env t ^^ set_val ^^
                remember_failure get_val ^^
                get_val
              end
              begin
                match normalize t with
                | Prim Null | Opt _ | Any -> Opt.null_lit env
                | _ -> coercion_failed "IDL error: did not find tuple field in record"
              end
          ) ts ^^

          (* skip all possible trailing extra fields *)
          get_typ_buf ^^ get_data_buf ^^ get_typtbl ^^ get_n_ptr ^^
          E.call_import env "rts" "skip_fields" ^^

          Tuple.from_stack env (List.length ts)
        )
      | Obj ((Object | Memory), fs) ->
        with_record_typ (fun get_typ_buf get_n_ptr ->
          let (set_val, get_val) = new_local env "val" in

          Object.lit_raw env (List.map (fun (h,f) ->
            f.Type.lab, fun () ->
              (* skip all possible intermediate extra fields *)
              get_typ_buf ^^ get_data_buf ^^ get_typtbl ^^ compile_const_32 (Lib.Uint32.to_int32 h) ^^ get_n_ptr ^^
              E.call_import env "rts" "find_field" ^^
              Bool.from_rts_int32 ^^
              E.if1 I64Type
                begin
                  ReadBuf.read_sleb128 env get_typ_buf ^^
                  go env f.typ ^^ set_val ^^
                  remember_failure get_val ^^
                  get_val
                  end
                begin
                  match normalize f.typ with
                  | Prim Null | Opt _ | Any -> Opt.null_lit env
                  | _ -> coercion_failed (Printf.sprintf "IDL error: did not find field %s in record" f.lab)
                end
          ) (sort_by_hash fs)) ^^

          (* skip all possible trailing extra fields *)
          get_typ_buf ^^ get_data_buf ^^ get_typtbl ^^ get_n_ptr ^^
          E.call_import env "rts" "skip_fields"
          )
      | Array (Mut t) ->
        read_alias env (Array (Mut t)) (fun get_array_typ on_alloc ->
          let (set_len, get_len) = new_local env "len" in
          let (set_x, get_x) = new_local env "x" in
          let (set_val, get_val) = new_local env "val" in
          let (set_arg_typ, get_arg_typ) = new_local env "arg_typ" in
          (* TODO: if possible refactor to match new Array t code,
             (perhaps too risky and unnecessary for extended candid due to lack of fancy opt subtyping, see #4243)
          *)
          with_composite_arg_typ get_array_typ idl_vec (ReadBuf.read_sleb128 env) ^^ set_arg_typ ^^
          ReadBuf.read_leb128 env get_data_buf ^^ set_len ^^
          Arr.alloc env Tagged.M get_len ^^ set_x ^^
          on_alloc get_x ^^
          get_len ^^ from_0_to_n env (fun get_i ->
            get_x ^^ get_i ^^ Arr.unsafe_idx env ^^
            get_arg_typ ^^ go env t ^^ set_val ^^
            remember_failure get_val ^^
            get_val ^^ store_ptr
          ) ^^
          get_x ^^
          Tagged.allocation_barrier env ^^
          G.i Drop
        )
      | Prim Region ->
         read_alias env (Prim Region) (fun get_region_typ on_alloc ->
          let (set_region, get_region) = new_local env "region" in
          (* sanity check *)
          get_region_typ ^^
          compile_eq_const (Wasm.I64_convert.extend_i32_s (Int32.neg (Option.get (to_idl_prim Candid (Prim Region))))) ^^
          E.else_trap_with env "deserialize_go (Region): unexpected idl_typ" ^^
          (* pre-allocate a region object, with dummy fields *)
          compile_unboxed_const 0L ^^ (* id *)
          compile_unboxed_const 0L ^^ (* pagecount *)
          Blob.lit env Tagged.B "" ^^ (* vec_pages *)
          Region.alloc_region env ^^
          set_region ^^
          on_alloc get_region ^^
          (* read and initialize the region's fields *)
          get_region ^^
          ReadBuf.read_word64 env get_data_buf ^^ (* id *)
          ReadBuf.read_word32 env get_data_buf ^^ (* pagecount *)
          read_blob () ^^ (* vec_pages *)
          Region.init_region env
        )
      | Array t ->
        let (set_len, get_len) = new_local env "len" in
        let (set_x, get_x) = new_local env "x" in
        let (set_val, get_val) = new_local env "val" in
        let (set_arg_typ, get_arg_typ) = new_local env "arg_typ" in
        with_composite_typ idl_vec (fun get_typ_buf ->
          ReadBuf.read_sleb128 env get_typ_buf ^^
          set_arg_typ ^^
          ReadBuf.read_leb128 env get_data_buf ^^ set_len ^^
          (* Don't decrement just check quota *)
          Bool.lit_rts_int32 false ^^
          get_len ^^
          Registers.idl_limit_check env ^^
          Arr.alloc env Tagged.I get_len ^^ set_x ^^
          get_len ^^ from_0_to_n env (fun get_i ->
          get_x ^^ get_i ^^ Arr.unsafe_idx env ^^
          get_arg_typ ^^ go env t ^^ set_val ^^
          remember_failure get_val ^^
          get_val ^^ store_ptr
        ) ^^
        get_x ^^
        Tagged.allocation_barrier env)
      | Opt t ->
        check_prim_typ (Prim Null) ^^
        E.if1 I64Type (Opt.null_lit env)
        begin
          check_prim_typ Any ^^ (* reserved *)
          E.if1 I64Type (Opt.null_lit env)
          begin
            check_composite_typ get_idltyp idl_opt ^^
            E.if1 I64Type
            begin
              let (set_arg_typ, get_arg_typ) = new_local env "arg_typ" in
              with_composite_typ idl_opt (ReadBuf.read_sleb128 env) ^^ set_arg_typ ^^
              read_byte_tagged
                [ Opt.null_lit env
                ; let (set_val, get_val) = new_local env "val" in
                  get_arg_typ ^^ go_can_recover env t ^^ set_val ^^
                  get_val ^^ compile_eq_const (coercion_error_value env) ^^
                  E.if1 I64Type
                    (* decoding failed, but this is opt, so: return null *)
                    (Opt.null_lit env)
                    (* decoding succeeded, return opt value *)
                    (Opt.inject env get_val)
                ]
            end
            begin
              (* Try constituent type *)
              let (set_val, get_val) = new_local env "val" in
              get_idltyp ^^ go_can_recover env t ^^ set_val ^^
              get_val ^^ compile_eq_const (coercion_error_value env) ^^
              E.if1 I64Type
                (* decoding failed, but this is opt, so: return null *)
                (Opt.null_lit env)
                (* decoding succeeded, return opt value *)
                (Opt.inject env get_val)
            end
          end
        end
      | Variant vs ->
        let (set_val, get_val) = new_local env "val" in
        with_composite_typ idl_variant (fun get_typ_buf ->
          (* Find the tag *)
          let (set_n, get_n) = new_local env "len" in
          ReadBuf.read_leb128 env get_typ_buf ^^ set_n ^^

          let (set_tagidx, get_tagidx) = new_local env "tagidx" in
          ReadBuf.read_leb128 env get_data_buf ^^ set_tagidx ^^

          get_tagidx ^^ get_n ^^
          compile_comparison I64Op.LtU ^^
          E.else_trap_with env "IDL error: variant index out of bounds" ^^

          (* Zoom past the previous entries *)
          get_tagidx ^^ from_0_to_n env (fun _ ->
            get_typ_buf ^^ E.call_import env "rts" "skip_leb128" ^^
            get_typ_buf ^^ E.call_import env "rts" "skip_leb128"
          ) ^^

          (* Now read the tag *)
          let (set_tag, get_tag) = new_local env "tag" in
          ReadBuf.read_leb128 env get_typ_buf ^^ set_tag ^^
          let (set_arg_typ, get_arg_typ) = new_local env "arg_typ" in
          ReadBuf.read_sleb128 env get_typ_buf ^^ set_arg_typ ^^

          List.fold_right (fun (h, {lab = l; typ = t; _}) continue ->
              get_tag ^^ compile_eq_const (Wasm.I64_convert.extend_i32_u (Lib.Uint32.to_int32 h)) ^^
              E.if1 I64Type
                ( Variant.inject env l (
                  get_arg_typ ^^ go env t ^^ set_val ^^
                  remember_failure get_val ^^
                  get_val
                ))
                continue
            )
            ( sort_by_hash vs )
            ( skip get_arg_typ ^^
              coercion_failed "IDL error: unexpected variant tag" )
        )
      | Func _ ->
        (* See Note [Candid subtype checks] *)
        get_rel_buf_opt ^^
        E.if1 I64Type
          begin
            get_idltyp ^^
            idl_sub env t
          end
          (Bool.lit true) ^^ (* if we don't have a subtype memo table, assume the types are ok *)
        E.if1 I64Type
          (with_composite_typ idl_func (fun _get_typ_buf ->
            read_byte_tagged
              [ E.trap_with env "IDL error: unexpected function reference"
              ; let (set_actor, get_actor) = new_local env "actor" in
                let (set_func, get_func) = new_local env "func" in
                read_actor_data () ^^ set_actor ^^
                read_text () ^^ set_func ^^
                Arr.lit env Tagged.S [get_actor; get_func]
              ]))
          (skip get_idltyp ^^
           coercion_failed "IDL error: incompatible function type")
      | Obj (Actor, _) ->
        (* See Note [Candid subtype checks] *)
        get_rel_buf_opt ^^
        E.if1 I64Type
          begin
            get_idltyp ^^
            idl_sub env t
          end
          (Bool.lit true) ^^
        E.if1 I64Type
          (with_composite_typ idl_service
             (fun _get_typ_buf -> read_actor_data ()))
          (skip get_idltyp ^^
           coercion_failed "IDL error: incompatible actor type")
      | Mut t ->
        read_alias env (Mut t) (fun get_arg_typ on_alloc ->
          let (set_result, get_result) = new_local env "result" in
          MutBox.alloc env ^^ set_result ^^
          on_alloc get_result ^^
          get_result ^^
          get_arg_typ ^^ go env t ^^
          MutBox.store_field env
          )
      | Weak t ->
        E.trap_with env "deserialize_go: Weak" ^^
        read_alias env (Weak t) (fun get_arg_typ on_alloc ->
          let (set_result, get_result) = new_local env "result" in
          WeakRef.alloc env ^^ set_result ^^
          on_alloc get_result ^^
          get_result ^^
          get_arg_typ ^^ go env t ^^
          WeakRef.store_field env
        )
      | Non ->
        skip get_idltyp ^^
        coercion_failed "IDL error: deserializing value of type None"
      | _ -> todo_trap env "deserialize" (Arrange_ir.typ t)
      end ^^
      (* Parsed value on the stack, return that, unless the failure flag is set *)
      when_failed (compile_unboxed_const (coercion_error_value env) ^^ G.i Return)
    )

  let serialize env ts : G.t =
    let name = Strm.name_for "serialize" ts in
    (* returns data/length pointers (will be GC’ed next time!) *)
    Func.share_code1 Func.Always env name ("x", I64Type) [I64Type; I64Type] (fun env get_x ->
      let (set_data_size, get_data_size) = new_local env "data_size" in
      let (set_refs_size, get_refs_size) = new_local env "refs_size" in

      let (tydesc, _offsets, _idltyps) = type_desc env Candid ts in
      let tydesc_len = Int64.of_int (String.length tydesc) in

      (* Get object sizes *)
      get_x ^^
      buffer_size env (Type.seq ts) ^^
      set_refs_size ^^
      set_data_size ^^
      (* check for overflow *)
      get_data_size ^^
      compile_add_const tydesc_len ^^
      compile_unboxed_const tydesc_len ^^
      compile_comparison I64Op.LtU ^^
      E.then_trap_with env "serialization overflow" ^^

      let (set_data_start, get_data_start) = new_local env "data_start" in
      let (set_refs_start, get_refs_start) = new_local env "refs_start" in

      (* Create a stream with suitable capacity and given header *)
      Strm.create env get_data_size set_data_start get_data_start tydesc ^^
      get_refs_size ^^ compile_mul_const Heap.word_size ^^ Blob.dyn_alloc_scratch env ^^ set_refs_start ^^

      (* Serialize x into the buffer *)
      get_x ^^
      get_data_start ^^
      get_refs_start ^^
      serialize_go env (Type.seq ts) ^^

      (* Sanity check: Did we fill exactly the buffer *)
      get_refs_start ^^ get_refs_size ^^ compile_mul_const Heap.word_size ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
      compile_comparison I64Op.Eq ^^
      E.else_trap_with env "reference buffer not filled" ^^

      (* Verify that the stream is correctly filled *)
      Strm.check_filled env get_data_start get_data_size ^^
      get_refs_size ^^
      compile_eq_const 0L ^^
      E.else_trap_with env "cannot send references on IC System API" ^^

      (* Extract the payload if possible *)
      Strm.terminate env get_data_start get_data_size tydesc_len
    )


  let deserialize_from_blob extended env ts =
    let ts_name = typ_seq_hash ts in
    let name =
      (* TODO(#3185): this specialization on `extended` seems redundant,
         removing it might simplify things *and* share more code in binaries.
         The only tricky bit might be the conditional Stack.dynamic_with_words bit... *)
      if extended
      then "@deserialize_extended<" ^ ts_name ^ ">"
      else "@deserialize<" ^ ts_name ^ ">" in
    Func.share_code2 Func.Always env name (("blob", I64Type), ("can_recover", I64Type)) (List.map (fun _ -> I64Type) ts) (fun env get_blob get_can_recover ->
      let (set_data_size, get_data_size) = new_local env "data_size" in
      let (set_refs_size, get_refs_size) = new_local env "refs_size" in
      let (set_data_start, get_data_start) = new_local env "data_start" in
      let (set_refs_start, get_refs_start) = new_local env "refs_start" in
      let (set_arg_count, get_arg_count) = new_local env "arg_count" in
      let (set_val, get_val) = new_local env "val" in

      get_blob ^^ Blob.len env ^^ set_data_size ^^
      get_blob ^^ Blob.payload_ptr_unskewed env ^^ set_data_start ^^

      (* Allocate space for the reference buffer and copy it *)
      compile_unboxed_const 0L ^^ set_refs_size (* none yet *) ^^

      (* Allocate space for out parameters of parse_idl_header *)
      Stack.with_words env "get_typtbl_size_ptr" 1L (fun get_typtbl_size_ptr ->
      Stack.with_words env "get_typtbl_ptr" 1L (fun get_typtbl_ptr ->
      Stack.with_words env "get_maintyps_ptr" 1L (fun get_maintyps_ptr ->

      (* Allocate space for out parameters of idl_alloc_typtbl *)
      Stack.with_words env "get_global_typtbl_ptr" 1L (fun get_global_typtbl_ptr ->
      Stack.with_words env "get_global_typtbl_end_ptr" 1L (fun get_global_typtbl_end_ptr ->
      Stack.with_words env "get_global_typtbl_size_ptr" 1L (fun get_global_typtbl_size_ptr ->

      (* Set up read buffers *)
      ReadBuf.alloc env (fun get_data_buf -> ReadBuf.alloc env (fun get_ref_buf ->

      ReadBuf.set_ptr get_data_buf get_data_start ^^
      ReadBuf.set_size get_data_buf get_data_size ^^
      ReadBuf.set_ptr get_ref_buf get_refs_start ^^
      ReadBuf.set_size get_ref_buf (get_refs_size ^^ compile_mul_const Heap.word_size) ^^

      (* Go! *)
      let tydesc, _, _ = type_desc env Candid ts in
      let tydesc_len = Int64.of_int (String.length tydesc) in
      let tydesc_tolerance =
        compile_unboxed_const tydesc_len ^^
        Registers.get_type_scaler env ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^
        Registers.get_type_bias env ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) in
      Bool.(lit extended ^^ to_rts_int32) ^^ get_data_buf ^^ tydesc_tolerance ^^ get_typtbl_ptr ^^ get_typtbl_size_ptr ^^ get_maintyps_ptr ^^
      E.call_import env "rts" "parse_idl_header" ^^

      (* Allocate global type table, if necessary for subtype checks *)
      (if extended then
         G.nop
       else begin
         get_global_candid_data env ^^
         get_global_type_offsets env ^^
         get_global_typtbl_ptr ^^ get_global_typtbl_end_ptr ^^ get_global_typtbl_size_ptr ^^
         E.call_import env "rts" "idl_alloc_typtbl"
      end) ^^

      (* Allocate memo table, if necessary *)
      with_rel_buf_opt env extended
        (get_typtbl_size_ptr ^^ load_unskewed_ptr)
        (get_global_typtbl_size_ptr ^^ load_unskewed_ptr)
        (fun get_rel_buf_opt ->
      begin
        (* set up invariant register arguments *)
        get_rel_buf_opt ^^ Registers.set_rel_buf_opt env ^^
        get_data_buf ^^ Registers.set_data_buf env ^^
        get_ref_buf ^^ Registers.set_ref_buf env ^^
        get_typtbl_ptr ^^ load_unskewed_ptr ^^ Registers.set_typtbl env ^^
        get_maintyps_ptr ^^ load_unskewed_ptr ^^ Registers.set_typtbl_end env ^^
        get_typtbl_size_ptr ^^ load_unskewed_ptr ^^ Registers.set_typtbl_size env ^^
        get_global_typtbl_ptr ^^ load_unskewed_ptr ^^ Registers.set_global_typtbl env ^^
        get_global_typtbl_end_ptr ^^ load_unskewed_ptr ^^ Registers.set_global_typtbl_end env ^^
        get_global_typtbl_size_ptr ^^ load_unskewed_ptr ^^ Registers.set_global_typtbl_size env ^^
        Registers.reset_value_limit env get_blob get_rel_buf_opt
      end ^^

      (* set up a dedicated read buffer for the list of main types *)
      ReadBuf.alloc env (fun get_main_typs_buf ->
        ReadBuf.set_ptr get_main_typs_buf (get_maintyps_ptr ^^ load_unskewed_ptr) ^^
        ReadBuf.set_end get_main_typs_buf (ReadBuf.get_end get_data_buf) ^^
        ReadBuf.read_leb128 env get_main_typs_buf ^^ set_arg_count ^^

        G.concat_map (fun t ->
          let can_recover, argument_default_or_trap, coercion_default_or_trap = Type.(
            let null_result _ = Opt.null_lit env in
            match normalize t with
            | Prim Null ->
              (get_can_recover, null_result, fun _ -> compile_unboxed_const (coercion_error_value env))
            | Opt _ | Any ->
              (Bool.lit true, null_result, null_result)
            | _ ->
              let default_or_trap msg =
                get_can_recover ^^
                E.if1 I64Type
                   (compile_unboxed_const (coercion_error_value env))
                   (E.trap_with env msg) in
              (get_can_recover, default_or_trap, default_or_trap))
          in
          get_arg_count ^^
          compile_eq_const 0L ^^
          E.if1 I64Type
           (argument_default_or_trap ("IDL error: too few arguments " ^ ts_name))
           (begin
              (* set up variable frame arguments *)
              Stack.with_frame env "frame_ptr" 3L (fun () ->
                (* idltyp *)
                ReadBuf.read_sleb128 env get_main_typs_buf ^^
                Stack.set_local env StackArgs.idltyp ^^
                (* depth *)
                compile_unboxed_const 0L ^^
                Stack.set_local env StackArgs.depth ^^
                (* recovery mode *)
                can_recover ^^
                Stack.set_local env StackArgs.can_recover ^^
                deserialize_go env t
             )
             ^^ set_val ^^
             get_arg_count ^^ compile_sub_const 1L ^^ set_arg_count ^^
             get_val ^^ compile_eq_const (coercion_error_value env) ^^
             (E.if1 I64Type
               (coercion_default_or_trap "IDL error: coercion failure encountered")
               get_val)
            end)
        ) ts ^^

        (* Skip any extra arguments *)
        compile_while env
         (get_arg_count ^^ compile_rel_const I64Op.GtU 0L)
         begin
           get_data_buf ^^
           get_typtbl_ptr ^^ load_unskewed_ptr ^^
           ReadBuf.read_sleb128 env get_main_typs_buf ^^
           G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
           compile_const_32 0l ^^
           E.call_import env "rts" "skip_any" ^^
           get_arg_count ^^ compile_sub_const 1L ^^ set_arg_count
         end ^^

        ReadBuf.is_empty env get_data_buf ^^
        E.else_trap_with env ("IDL error: left-over bytes " ^ ts_name) ^^
        ReadBuf.is_empty env get_ref_buf ^^
        E.else_trap_with env ("IDL error: left-over references " ^ ts_name) ^^

        (* Safety guard: The temporary pointers in the registers must no longer be used when a GC increment runs. *)
        Registers.clear_registers env
      )))))))))

    ))

  let deserialize env ts =
    IC.arg_data env ^^
    Bool.lit false ^^ (* can't recover *)
    deserialize_from_blob false env ts

(*
Note [speculating for short (S)LEB encoded bignums]
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#3098 highlighted that a lot of heap garbage can be generated while reading in
(S)LEB-encoded bignums. To make heap consumption optimal for every compactly
representable `Int`, we resort to speculatively reading a 64-byte chunk from
the `ReadBuf`. We call it speculative, because it may read past the end of the
buffer (and thus end up containing junk bytes) or even fail because reading
across Wasm page boundaries could cause trapping. (Consider the buffer ending
3 bytes before the last-memory-page boundary and issuing a speculative 64-bit read for the
address 2 bytes less than buffer end.) In case of failure to read data, `-1`
(a sentinel) is returned. (The sentinel could be use-case specific when later
the need arises.)

In most cases the speculative read will come back with valid bytes. How many
of those are relevant, can be judged by consulting the buffer-end pointer or
analysing the 64-bit word directly. In the case of (S)LEB, the continuation and
termination bits can be filtered and thus the encoding's last byte detected when
present in the 64-bit word.

If such a LEB boundary is detected, avenues open up for a much faster (than
bytewise-sequential) parsing.

After the data is interpreted, it's the client's responsibility to adjust the
current buffer position.

 *)

(*
Note [mutable stable values]
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

We currently use a Candid derivative to serialize stable values. In addition to
storing sharable data, we can also store mutable data (records with mutable
fields and mutable arrays), and we need to preserve aliasing.

To that end we extend Candid with a type constructor `alias t`.

In the type table, alias t is represented by type code 1. All Candid type constructors
are represented by negative numbers, so this cannot clash with anything and,
conveniently, makes such values illegal Candid.

The values of `alias t` are either

 * i8(0) 0x00000000 0x00000000 M(v)
   for one (typically the first) occurrence of v
   The first 0x00000000 is the “memo field”, the second is the “type hash field”.
   Both are scratch spaces for the benefit of the decoder.
   We use **pointer compression** to store 64-bit pointer that are potentially larger
   than 4GB but small enough to fit into 32-bit with the compressed representation.
   Pointers are expected to refer to at most 8GB as the memory representation may grow
   up to two times by switching from 32-bit to 64-bit.

or

 * i8(1) i32(offset) M(v)
   for all other occurrences of v, where offset is the relative position of the
   above occurrences from this reference.

We map Motoko types to this as follows:

  e([var t]) = alias e([t]) = alias vec e(t)
  e({var field : t}) = record { field : alias e(t) }

Why different? Because we need to alias arrays as a whole (we can’t even alias
their fields, as they are manifestly part of the array heap structure), but
aliasing records does not work, as aliased record values may appear at
different types (due to subtyping), and Candid serialization is type-driven.
Luckily records put all mutable fields behind an indirection (MutBox), so this
works.

The type-driven code in this module treats `Type.Mut` to always refer to an
`MutBox`; for arrays the mutable case is handled directly.

To detect and preserve aliasing, these steps are taken:

 * In `buffer_size`, when we see a mutable thing (`Array` or `MutBox`), the
   first time, we mark it by setting the heap tag to `StableSeen`.
   This way, when we see it a second time, we can skip the value in the size
   calculation.
 * In `serialize`, when we see it a first time (tag still `StableSeen`),
   we serialize it (first form above), and remember the absolute position
   in the output buffer, abusing the heap tag here.
   (Invariant: This absolute position is never `StableSeen`)
   Upon a second visit (tag not `StableSeen`), we can thus fetch that absolute
   position and calculate the offset.
 * In `deserialize`, when we come across a `alias t`, we follow the offset (if
   needed) to find the content.

   If the memo field is still `0x00000000`, this is the first time we read
   this, so we deserialize to the Motoko heap, and remember the **compressed**
   64-bit vanilla pointer by overwriting the memo field.
   We also store the **compressed** pointer to a blob with the type hash of
   the type we are serializing at in the type hash field.

   NOTE for 64-bit destabilization: The Candid destabilization format historically
   only reserves 32-bit space for remembering addresses of aliases. However, when
   upgrading from old Candid destabilization to new enhanced orthogonal persistence,
   the deserialized objects may occupy larger object space (worst case the double space),
   such that pointers may be larger than 4GB. Therefore, we use pointer compression to
   safely narrow 64-bit addresses into 32-bit Candid(ish) memo space. The compression
   relies on the property that the 3 lower bits of the unskewed pointer are zero due
   to the 8-bit (64-bit) object alignment.

   If it is not `0x00000000` then we can simply read the **compressed** pointer
   from there, after checking the type hash field to make sure we are aliasing at
   the same type.

 *)

(*
Note [Candid subtype checks]
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Deserializing Candid values requires a Candid subtype check when
deserializing values of reference types (actors and functions).

The subtype test is performed directly on the expected and actual
candid type tables using RTS functions `idl_sub_buf_words`,
`idl_sub_buf_init` and `idl_sub`. One type table and vector of types
is generated statically from the list of statically known types
encountered during code generation, the other is determined
dynamically by, e.g. message payload. The latter will vary with
each payload to decode.

The static type table and a type descriptor are stored in passive
data segments. Instead of absolute memory addresses, the static type
table in the data segment only contains relative offsets into type
descriptor. When loaded, these offsets are patched by static addresses
that point into the type descriptor.

The known Motoko types are accumulated in a global list as required
and then, in a final compilation step, encoded to global type table
and the type descriptor (sequence of type indices). The encoding is
stored in passive data segments referenced (by way of segment indices)
from dedicated wasm globals so that we can generate code that
references the globals before their final definitions are known.

Deserializing a proper (not extended) Candid value stack allocates a
mutable word buffer, of size determined by `idl_sub_buf_words`.
The word buffer is used to initialize and provide storage for a
Rust memo table (see bitrel.rs) memoizing the result of sub and
super type tests performed during deserialization of a given Candid
value sequence. The memo table is initialized once, using `idl_sub_buf_init`,
then shared between recursive calls to deserialize, by threading the (possibly
null) wasm address of the word buffer as an optional argument. The
word buffer is stack allocated in generated code, not Rust, because
it's size is dynamic and Rust doesn't seem to support dynamically-sized
stack allocation.

Currently, we only perform Candid subtype checks when decoding proper
(not extended) Candid values. Extended values are required for
stable variables only: we can omit the check, because compatibility
should already be enforced by the static signature compatibility
check. We use the `null`-ness of the word buffer pointer to
dynamically determine whether to omit or perform Candid subtype checks.

NB: Extending `idl_sub` to support extended, "stable" types (with mutable,
invariant type constructors) would require extending the polarity argument
from a Boolean to a three-valued argument to efficiently check equality for
invariant type constructors in a single pass.
*)

end (* Serialization *)

(* OldStabilization as migration code:
  Deserializing a last time from Candid-serialized stable objects into the stable heap:
   * stable variables; and
   * virtual stable memory.
   c.f.
   * ../../design/Stable.md
   * ../../design/StableMemory.md
*)
module OldStabilization = struct
  let load_word32 = G.i (Load {ty = I32Type; align = 0; offset = 0L; sz = None})
  let store_word32 = G.i (Store {ty = I32Type; align = 0; offset = 0L; sz = None})
  let write_word32 env = StableMem.write env false "word32" I32Type 4L store_word32

  (* read and clear word32 from stable mem offset on stack *)
  let read_and_clear_word32 env =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.share_code1 Func.Always env "__stablemem_read_and_clear_word32"
        ("offset", I64Type) [I64Type]
        (fun env get_offset ->
          Stack.with_words env "temp_ptr" 1L (fun get_temp_ptr ->
            let (set_word, get_word) = new_local32 env "word" in
            (* read word *)
            get_temp_ptr ^^
            get_offset ^^
            compile_unboxed_const 4L ^^
            StableMem.stable64_read env ^^
            get_temp_ptr ^^ load_word32 ^^
            set_word ^^
            (* write 0 *)
            get_temp_ptr ^^ compile_const_32 0l ^^ store_word32 ^^
            get_offset ^^
            get_temp_ptr ^^
            compile_unboxed_const 4L ^^
            StableMem.stable64_write env ^^
            (* return word *)
            get_word ^^
            G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32))
        ))
    | _ -> assert false

  (* TODO: rewrite using MemoryFill *)
  let blob_clear env =
    Func.share_code1 Func.Always env "blob_clear" ("x", I64Type) [] (fun env get_x ->
      let (set_ptr, get_ptr) = new_local env "ptr" in
      let (set_len, get_len) = new_local env "len" in
      get_x ^^
      Blob.as_ptr_len env ^^
      set_len ^^
      set_ptr ^^

      (* round to word size *)
      get_len ^^
      compile_add_const (Int64.sub Heap.word_size 1L) ^^
      compile_divU_const Heap.word_size ^^

      (* clear all words *)
      from_0_to_n env (fun get_i ->
        get_ptr ^^
        compile_unboxed_const 0L ^^
        store_unskewed_ptr ^^
        get_ptr ^^
        compile_add_const Heap.word_size ^^
        set_ptr))

  let candid_destabilize env ty save_version =
    match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      let (set_pages, get_pages) = new_local env "pages" in
      StableMem.stable64_size env ^^
      set_pages ^^

      get_pages ^^
      compile_test I64Op.Eqz ^^
      E.if1 I64Type
        begin
          (* Case: Size zero ==> Nothing in stable memory,
             so result becomes the nil-valued record. *)
          let (_, fs) = Type.as_obj ty in
          let fs' = List.map
           (fun f -> (f.Type.lab, fun () -> Opt.null_lit env))
           fs
          in
          StableMem.get_mem_size env ^^
          compile_test I64Op.Eqz ^^
          E.else_trap_with env "StableMem.mem_size non-zero" ^^
          compile_unboxed_const StableMem.version_stable_heap_no_regions ^^
          save_version ^^
          Object.lit_raw env fs'
        end
        begin
          (* Case: Non-zero size. *)
          let (set_marker, get_marker) = new_local env "marker" in
          let (set_len, get_len) = new_local env "len" in
          let (set_offset, get_offset) = new_local env "offset" in
          compile_unboxed_const 0L ^^
          read_and_clear_word32 env ^^
          set_marker ^^

          get_marker ^^
          compile_test I64Op.Eqz ^^
          E.if0
            begin
              (* Sub-Case: version 1 or 2:
                 Regions/Experimental API and stable vars. *)
              let (set_M, get_M) = new_local env "M" in
              let (set_version, get_version) = new_local env "version" in
              let (set_N, get_N) = new_local env "N" in

              StableMem.stable64_size env ^^
              compile_sub_const 1L ^^
              compile_shl_const (Int64.of_int page_size_bits) ^^
              set_M ^^

              (* read version *)
              get_M ^^
              compile_add_const (Int64.sub page_size 4L) ^^
              read_and_clear_word32 env ^^
              set_version ^^
              get_version ^^
              save_version ^^

              (* check version *)
              get_version ^^
              compile_unboxed_const (StableMem.version_max) ^^
              compile_comparison I64Op.GtU ^^
              E.then_trap_with env (Printf.sprintf
                "higher stable memory version (expected 1..%s)"
                (Int64.to_string StableMem.version_max)) ^^

              (* restore StableMem bytes [0..4) *)
              compile_unboxed_const 0L ^^
              get_M ^^
              compile_add_const (Int64.sub page_size 8L) ^^
              read_and_clear_word32 env ^^
              G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
              write_word32 env ^^

              (* restore mem_size *)
              get_M ^^
              compile_add_const (Int64.sub page_size 12L) ^^
              read_and_clear_word32 env ^^ (*TODO: use 64 bits *)
              StableMem.set_mem_size env ^^

              StableMem.get_mem_size env ^^
              compile_shl_const (Int64.of_int page_size_bits) ^^
              set_N ^^

              (* set len *)
              get_N ^^
              read_and_clear_word32 env ^^
              set_len ^^

              (* set offset *)
              get_N ^^
              compile_add_const 4L ^^
              set_offset
            end
            begin
              (* Sub-Case: Version 0.
                 Stable vars with NO Regions/Experimental API. *)
              (* assert mem_size == 0 *)
              StableMem.get_mem_size env ^^
              compile_test I64Op.Eqz ^^
              E.else_trap_with env "unexpected, non-zero stable memory size" ^^

              (* set len *)
              get_marker ^^
              set_len ^^

              (* set offset *)
              compile_unboxed_const 4L ^^
              set_offset ^^

              compile_unboxed_const (Int64.of_int 0) ^^
              save_version
            end ^^ (* if_ *)

          let (set_blob, get_blob) = new_local env "blob" in
          (* read blob from stable memory *)
          Blob.alloc env Tagged.B get_len ^^ set_blob ^^
          get_blob ^^ Blob.payload_ptr_unskewed env ^^
          get_offset ^^
          get_len ^^
          StableMem.stable64_read env ^^

          let (set_val, get_val) = new_local env "val" in
          (* deserialize blob to val *)
          get_blob ^^
          Bool.lit false ^^ (* can't recover *)
          Serialization.deserialize_from_blob true env [ty] ^^
          set_val ^^

          (* clear blob contents *)
          get_blob ^^
          blob_clear env ^^

          (* copy zeros from blob to stable memory *)
          get_offset ^^
          get_blob ^^ Blob.payload_ptr_unskewed env ^^
          get_blob ^^ Blob.len env ^^
          StableMem.stable64_write env ^^

          (* return val *)
          get_val
        end
    | _ -> assert false

  let load env actor_type upgrade_version =
    candid_destabilize env actor_type upgrade_version ^^
    UpgradeStatistics.add_instructions env
end

(* New stable memory layout with dedicated version for enhanced orthogonal persistence.
   This prevents unwanted forward compatibility of old compiled programs that rely on Candid destabilization.
   This also helps to detect graph-copy-based destabilization that has priority over enhanced orthogonal persistence.
  If size == 0: empty
  let end = physical size * page_size
  If logical size N > 0:
    [0..4)          0 (first word is backed up at `end-8`)
    [4..N)          <stable memory>
            <zero padding>
    [end-16..end-8) <size N>
    [end-8..end-4)  <first word>
    [end-4..end)   <new version>
  ending at page boundary
  Note: The first word must be empty to distinguish this version from the Candid legacy version 0 (which has first word != 0).
*)
module NewStableMemory = struct
  let physical_size env =
    IC.system_call env "stable64_size" ^^
    compile_shl_const (Int64.of_int page_size_bits)

  let store_at_end env offset typ get_value =
    physical_size env ^^
    compile_sub_const offset ^^
    get_value ^^
    match typ with
    | I32Type -> StableMem.write_word32 env
    | I64Type -> StableMem.write_word64 env
    | _ -> assert false

  let read_from_end env offset typ =
    physical_size env ^^
    compile_sub_const offset ^^
    match typ with
    | I32Type -> StableMem.read_word32 env
    | I64Type -> StableMem.read_word64 env
    | _ -> assert false

  let clear_at_end env offset typ =
    store_at_end env offset typ
    (match typ with
    | I32Type -> compile_const_32 0l
    | I64Type -> compile_unboxed_const 0L
    | _ -> assert false
    )

  let logical_size_offset = 16L
  let first_word_backup_offset = 8L
  let version_offset = 4L

  let upgrade_version_from_candid env =
    StableMem.set_version env ^^
    StableMem.get_version env ^^
    compile_eq_const StableMem.legacy_version_no_stable_memory ^^
    StableMem.get_version env ^^
    compile_eq_const StableMem.legacy_version_some_stable_memory ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
    E.if1 I64Type
    begin
      compile_unboxed_const StableMem.version_stable_heap_no_regions
    end
    begin
      StableMem.get_version env ^^
      compile_eq_const StableMem.legacy_version_regions ^^
      E.else_trap_with env "Unsupported stable memory version when upgrading from Candid stabilization" ^^
      compile_unboxed_const StableMem.version_stable_heap_regions
    end ^^
    StableMem.set_version env


  let upgrade_version_from_graph_stabilization env =
    StableMem.get_version env ^^
    compile_eq_const StableMem.version_graph_copy_no_regions ^^
    E.if1 I64Type
    begin
      compile_unboxed_const StableMem.version_stable_heap_no_regions
    end
    begin
      StableMem.get_version env ^^
      compile_eq_const StableMem.version_graph_copy_regions ^^
      E.else_trap_with env "Unsupported stable memory version when upgrading from graph-copy-based stabilization" ^^
      compile_unboxed_const StableMem.version_stable_heap_regions
    end ^^
    StableMem.set_version env

  let grow_size env amount =
    StableMem.get_mem_size env ^^
    compile_shl_const (Int64.of_int page_size_bits) ^^
    compile_unboxed_const amount ^^
    StableMem.ensure env

  let backup env =
    let (set_first_word, get_first_word) = new_local32 env "first_word" in
    physical_size env ^^
    compile_test I64Op.Eqz ^^
    E.if0
      G.nop
      begin
        (* read and clear first word *)
        compile_unboxed_const 0L ^^ StableMem.read_word32 env ^^ set_first_word ^^
        compile_unboxed_const 0L ^^ compile_const_32 0l ^^ StableMem.write_word32 env ^^

        grow_size env logical_size_offset ^^

        (* backup logical size *)
        store_at_end env logical_size_offset I64Type (StableMem.get_mem_size env) ^^

        (* backup first word *)
        store_at_end env first_word_backup_offset I32Type get_first_word ^^

        (* store the version *)
        store_at_end env version_offset I32Type (StableMem.get_version env ^^ G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)))
      end

  let restore env =
    let (set_first_word, get_first_word) = new_local32 env "first_word" in
    physical_size env ^^
    compile_test I64Op.Eqz ^^
    E.if0
      begin
        compile_unboxed_const 0L ^^ StableMem.set_mem_size env
      end
      begin
        (* check the version *)
        read_from_end env version_offset I32Type ^^
        G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
        StableMem.set_version env ^^
        StableMem.get_version env ^^
        compile_eq_const StableMem.version_stable_heap_no_regions ^^
        StableMem.get_version env ^^
        compile_eq_const StableMem.version_stable_heap_regions ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        E.else_trap_with env (Printf.sprintf
          "unsupported stable memory version (expected %s or %s)"
           (Int64.to_string StableMem.version_stable_heap_no_regions)
           (Int64.to_string StableMem.version_stable_heap_regions)) ^^

        (* read first word *)
        read_from_end env first_word_backup_offset I32Type ^^
        set_first_word ^^

        (* restore logical size *)
        read_from_end env logical_size_offset I64Type ^^
        StableMem.set_mem_size env ^^

        (* clear size and version *)
        clear_at_end env logical_size_offset I64Type ^^
        clear_at_end env first_word_backup_offset I32Type ^^
        clear_at_end env version_offset I32Type ^^

        (* restore first word *)
        compile_unboxed_const 0L ^^ get_first_word ^^ StableMem.write_word32 env
      end
end

(* Enhanced orthogonal persistence *)
module EnhancedOrthogonalPersistence = struct

  let has_stable_actor env = E.call_import env "rts" "has_stable_actor"

  let load_stable_actor env = E.call_import env "rts" "load_stable_actor"

  let save_stable_actor env = E.call_import env "rts" "save_stable_actor"

  let free_stable_actor env = E.call_import env "rts" "free_stable_actor"

  let create_type_descriptor env actor_type =
    let (candid_type_desc, type_offsets, type_indices) = Serialization.(type_desc env Persistence [actor_type]) in
    let serialized_offsets = StaticBytes.(as_bytes [i64s (List.map Int64.of_int type_offsets)]) in
    assert (type_indices = [0l]);
    Blob.lit env Tagged.B candid_type_desc ^^
    Blob.lit env Tagged.B serialized_offsets

  let register_stable_type env actor_type =
    create_type_descriptor env actor_type ^^
    E.call_import env "rts" "register_stable_type"

  let assign_stable_type env actor_type =
    create_type_descriptor env actor_type ^^
    E.call_import env "rts" "assign_stable_type"

  let load_old_field env field get_old_actor =
    if field.Type.typ = Type.(Opt Any) then
      (* A stable variable may have been promoted to type `Any`: Therefore, drop its former content. *)
      Opt.inject env (Tuple.compile_unit env)
    else
      (get_old_actor ^^ Object.load_idx_raw env field.Type.lab)

  let empty_actor env actor_type =
    let (_, field_declarations) = Type.as_obj actor_type in
    let field_initializers = List.map
      (fun field -> (field.Type.lab, fun () -> Opt.null_lit env))
      field_declarations
    in
    Object.lit_raw env field_initializers

  (* Support additional fields in an upgraded actor. *)
  let upgrade_actor env actor_type =
    let set_old_actor, get_old_actor = new_local env "old_actor" in
    let get_field_value field =
      get_old_actor ^^
      Object.contains_field env field.Type.lab ^^
      (E.if1 I64Type
        (load_old_field env field get_old_actor)
        (Opt.null_lit env)
      ) in
    let (_, field_declarations) = Type.as_obj actor_type in
    let field_initializers = List.map
      (fun field -> (field.Type.lab, fun () -> (get_field_value field)))
      field_declarations
    in
    set_old_actor ^^
    Object.lit_raw env field_initializers

  let recover_actor env actor_type =
    load_stable_actor env ^^
    upgrade_actor env actor_type ^^
    free_stable_actor env

  let save env actor_type =
    assign_stable_type env actor_type ^^
    IC.get_actor_to_persist env ^^
    save_stable_actor env ^^
    NewStableMemory.backup env ^^
    UpgradeStatistics.set_instructions env

  let load env actor_type =
    register_stable_type env actor_type ^^
    load_stable_actor env ^^
    compile_test I64Op.Eqz ^^
    (E.if1 I64Type
      (empty_actor env actor_type)
      (recover_actor env actor_type)
    ) ^^
    NewStableMemory.restore env ^^
    UpgradeStatistics.add_instructions env

  let initialize env actor_type =
    register_stable_type env actor_type

end (* EnhancedOrthogonalPersistence *)

(* As fallback when doing persistent memory layout changes. *)
module GraphCopyStabilization = struct
  let is_graph_stabilization_started env =
    E.call_import env "rts" "is_graph_stabilization_started" ^^ Bool.from_rts_int32

  let start_graph_stabilization env actor_type =
    EnhancedOrthogonalPersistence.create_type_descriptor env actor_type ^^
    E.call_import env "rts" "start_graph_stabilization"

  let graph_stabilization_increment env =
    E.call_import env "rts" "graph_stabilization_increment" ^^ Bool.from_rts_int32

  let start_graph_destabilization env actor_type =
    EnhancedOrthogonalPersistence.create_type_descriptor env actor_type ^^
    E.call_import env "rts" "start_graph_destabilization"

  let graph_destabilization_increment env =
    E.call_import env "rts" "graph_destabilization_increment" ^^ Bool.from_rts_int32

  let get_graph_destabilized_actor env actor_type =
    E.call_import env "rts" "get_graph_destabilized_actor" ^^
    EnhancedOrthogonalPersistence.upgrade_actor env actor_type
end

module GCRoots = struct
  let register_static_variables env =
    E.(env.object_pool.frozen) := true;
    Func.share_code0 Func.Always env "initialize_root_array" [] (fun env ->
      let length = Int64.of_int (E.object_pool_size env) in
      compile_unboxed_const length ^^
      E.call_import env "rts" "initialize_static_variables" ^^
      E.iterate_object_pool env (fun index allocation ->
      Func.share_code0 Func.Always env (Printf.sprintf "alloc_%i" index) [] (fun env ->
            compile_unboxed_const (Int64.of_int index) ^^
            allocation env ^^
              E.call_import env "rts" "set_static_variable")
      )
    )
end (* GCRoots *)

(* This comes late because it also deals with messages *)
module FuncDec = struct
  let bind_args env ae0 first_arg args =
    let rec go i ae = function
    | [] -> ae
    | a::args ->
      (* Function arguments are always vanilla, due to subtyping and uniform representation.
         We keep them as such here for now. We _could_ always unpack those that can be unpacked
         (Nat32 etc.). It is generally hard to predict which strategy is better. *)
      let ae' = VarEnv.add_local_local env ae a.it SR.Vanilla (Int32.of_int i) a.note in
      go (i+1) ae' args in
    go first_arg ae0 args

  (* Create a WebAssembly func from a pattern (for the argument) and the body.
   Parameter `captured` should contain the, well, captured local variables that
   the function will find in the closure. *)
  let compile_local_function outer_env outer_ae restore_env args mk_body ret_tys at =
    let arg_names = List.map (fun a -> a.it, I64Type) args in
    let return_arity = List.length ret_tys in
    let retty = Lib.List.make return_arity I64Type in
    let ae0 = VarEnv.mk_fun_ae outer_ae in
    Func.of_body outer_env (["clos", I64Type] @ arg_names) retty (fun env -> G.with_region at (
      let get_closure = G.i (LocalGet (nr 0l)) ^^ Tagged.load_forwarding_pointer env in

      let ae1, closure_codeW = restore_env env ae0 get_closure in

      (* Add arguments to the environment (shifted by 1) *)
      let ae2 = bind_args env ae1 1 args in

      closure_codeW (mk_body env ae2)
    ))

  let message_start env sort = match sort with
      | Type.(Shared Write) ->
        Lifecycle.(trans env InUpdate)
      | Type.(Shared Query) ->
        Lifecycle.(trans env InQuery)
      | Type.(Shared Composite) ->
        Lifecycle.(trans env InComposite)
      | _ -> assert false

  let message_cleanup env sort = match sort with
      | Type.(Shared Write) ->
        Lifecycle.get env ^^
        compile_eq_const (Lifecycle.(int_of_state InStabilization)) ^^
        Lifecycle.get env ^^
        compile_eq_const (Lifecycle.(int_of_state InDestabilization)) ^^
        G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
        E.if0
          G.nop
          begin
            GC.collect_garbage env ^^
            Lifecycle.(trans env Idle)
          end
      | Type.Shared Type.Query ->
        Lifecycle.(trans env PostQuery)
      | Type.Shared Type.Composite ->
        (* Stay in composite query state such that callbacks of
        composite queries can also use the memory reserve.
        The state is isolated since memory changes of queries
        are rolled back by the IC runtime system. *)
        Lifecycle.(trans env InComposite)
      | _ -> assert false

  let callback_start env =
    Lifecycle.(is_in env InComposite) ^^
    E.if0
      (G.nop)
      (message_start env Type.(Shared Write))

  let callback_cleanup env =
    Lifecycle.(is_in env InComposite) ^^
    E.if0
      (G.nop)
      (message_cleanup env (Type.Shared Type.Write))

  let compile_const_message outer_env outer_ae sort control args mk_body ret_tys at exported_name_opt : E.func_with_names =
    let ae0 = VarEnv.mk_fun_ae outer_ae in
    
    (* Check if this is a raw HTTP handler method *)
    let is_raw_http_handler = match exported_name_opt with
      | Some name when (name = "http_request_v2" || name = "http_request_update_v2") ->
        (* Validate signature: must be (Blob|[Nat8]) -> async (Blob|[Nat8]) *)
        (match args with
         | [arg] ->
           let arg_type = Type.normalize arg.note in
           let unwrapped_type = Type.as_immut arg_type in
           (match unwrapped_type with
            | Type.Array elem_type ->
              (match Type.normalize elem_type with
               | Type.Prim Type.Nat8 -> true
               | _ -> false)
            | Type.Prim Type.Blob -> true
            | _ -> false)
         | _ -> false)
      | _ -> false
    in
    
    Func.of_body outer_env [] [] (fun env -> G.with_region at (
      message_start env sort ^^
      (* cycles *)
      Internals.reset_cycles env outer_ae ^^
      Internals.reset_refund env outer_ae ^^
      
      if is_raw_http_handler then begin
        (* RAW HTTP HANDLER PATH - Skip Candid deserialization on input *)
        let arg = List.hd args in
        let arg_ty = arg.note in
        let arg_name = arg.it in
        
        let arg_is_array = 
          let unwrapped = Type.as_immut (Type.normalize arg_ty) in
          match unwrapped with
          | Type.Array elem_type ->
            (match Type.normalize elem_type with
             | Type.Prim Type.Nat8 -> true
             | _ -> false)
          | _ -> false
        in
        
        let ae1 = VarEnv.add_argument_locals env ae0 [(arg_name, arg_ty)] in
        
        (* Mark this context as a raw HTTP handler so ICReplyPrim can skip serialization *)
        let old_raw_handler = !E.(env.compiling_raw_handler) in
        E.(env.compiling_raw_handler := true);
        
        (* Get raw IC argument data (no Candid deserialization) *)
        let code = 
          IC.arg_data env ^^
          (if arg_is_array then Arr.ofBlob env Tagged.I else G.nop) ^^
          Var.set_val_vanilla_from_stack env ae1 arg_name ^^
          
          (* Execute function body *)
          (* For async functions, the reply happens via ICReplyPrim which will
             check the raw_handler flag and skip serialization *)
          mk_body env ae1 ^^
          
          message_cleanup env sort
        in
        
        (* Restore the flag *)
        E.(env.compiling_raw_handler := old_raw_handler);
        code
      end else begin
        (* Regular path -- old code with Candid de/serialization. *)
        (* reply early for a oneway *)
        (if control = Type.Returns
         then
           Tuple.compile_unit env ^^
           Serialization.serialize env [] ^^
           IC.reply_with_data env
         else G.nop) ^^
        (* Deserialize argument and add params to the environment *)
        let arg_list = List.map (fun a -> (a.it, a.note)) args in
        let arg_names = List.map (fun a -> a.it) args in
        let arg_tys = List.map (fun a -> a.note) args in
        let ae1 = VarEnv.add_argument_locals env ae0 arg_list in
        Serialization.deserialize env arg_tys ^^
        G.concat_map (Var.set_val_vanilla_from_stack env ae1) (List.rev arg_names) ^^
        mk_body env ae1 ^^
        message_cleanup env sort
      end
    ))

  (* Compile a closed function declaration (captures no local variables) *)
  let closed pre_env sort control name args mk_body fun_rhs ret_tys at exported_name_opt =
    if Type.is_shared_sort sort
    then begin
      let (fi, fill) = E.reserve_fun pre_env name in
      ( Const.Message fi, fun env ae ->
        fill (compile_const_message env ae sort control args mk_body ret_tys at exported_name_opt)
      )
    end else begin
      assert (control = Type.Returns);
      let lf = E.make_lazy_function pre_env name in
      let fun_id = E.get_constant_function_id pre_env in
      ( Const.Fun (fun_id, (fun () -> Lib.AllocOnUse.use lf), fun_rhs), fun env ae ->
        let restore_no_env _env ae _ = ae, unmodified in
        Lib.AllocOnUse.def lf (lazy (compile_local_function env ae restore_no_env args mk_body ret_tys at))
      )
    end

  (* Compile a closure declaration (captures local variables) *)
  let closure env ae sort control name captured args mk_body ret_tys at =
      let is_local = sort = Type.Local in

      let set_clos, get_clos = new_local env (name ^ "_clos") in

      let len = Wasm.I64.of_int_u (List.length captured) in
      let store_env, restore_env =
        let rec go i = function
          | [] -> (G.nop, fun _env ae1 _ -> ae1, unmodified)
          | (v::vs) ->
              let store_rest, restore_rest = go (i + 1) vs in
              let store_this, restore_this = Var.capture env ae v in
              let store_env =
                get_clos ^^
                store_this ^^
                Closure.store_data env (Wasm.I64.of_int_u i) ^^
                store_rest in
              let restore_env env ae1 get_env =
                let ae2, codeW = restore_this env ae1 in
                let ae3, code_restW = restore_rest env ae2 get_env in
                (ae3,
                 fun body ->
                 get_env ^^
                 Closure.load_data env (Wasm.I64.of_int_u i) ^^
                 codeW (code_restW body)
                )
              in store_env, restore_env in
        go 0 captured in

      let f =
        if is_local
        then compile_local_function env ae restore_env args mk_body ret_tys at
        else assert false (* no first class shared functions yet *) in

      let fi = E.add_fun env name f in

      let code =
        (* Allocate a heap object for the closure *)
        Tagged.alloc env (Int64.add Closure.header_size len) Tagged.Closure ^^
        set_clos ^^

        (* Store the function pointer number: *)
        get_clos ^^
        compile_unboxed_const (Wasm.I64_convert.extend_i32_u (E.add_fun_ptr env fi)) ^^
        Tagged.store_field env Closure.funptr_field ^^

        (* Store the length *)
        get_clos ^^
        compile_unboxed_const len ^^
        Tagged.store_field env Closure.len_field ^^

        (* Store all captured values *)
        store_env ^^

        get_clos ^^
        Tagged.allocation_barrier env ^^
        G.i Drop
      in

      if is_local
      then
        SR.Vanilla,
        code ^^
        get_clos
      else assert false (* no first class shared functions *)

  let lit env ae name sort control free_vars args mk_body ret_tys at =
    let captured = List.filter (VarEnv.needs_capture ae) free_vars in

    if ae.VarEnv.lvl = VarEnv.TopLvl then assert (captured = []);

    if captured = []
    then
      let (ct, fill) = closed env sort control name args mk_body Const.Complicated ret_tys at None in
      fill env ae;
      (SR.Const ct, G.nop)
    else closure env ae sort control name captured args mk_body ret_tys at

  (* Returns a closure corresponding to a future (async block) *)
  let async_body env ae ts free_vars mk_body at =
    (* We compile this as a local, returning function, so set return type to [] *)
    let sr, code = lit env ae "anon_async" Type.Local Type.Returns free_vars [] mk_body [] at in
    code ^^
    StackRep.adjust env sr SR.Vanilla

  (* Takes the reply and reject callbacks, tuples them up (with administrative extras),
     adds them to the continuation table, and returns the two callbacks expected by
     ic.call_new.

     The tupling is necessary because we want to free _both_/_all_ closures
     when the call is answered.

     The reply callback function exists once per type (as it has to do
     deserialization); the reject callback function is unique.
  *)

  let closures_to_reply_reject_callbacks_aux env ts_opt =
    let arity, reply_name, from_arg_data =
      match ts_opt with
      | Some ts ->
        (List.length ts,
         "@callback<" ^ Typ_hash.typ_hash (Type.Tup ts) ^ ">",
         fun env -> Serialization.deserialize env ts)
      | None ->
        (1,
         "@callback",
         (fun env ->
           Blob.of_size_copy env Tagged.B
           (fun env ->
            IC.system_call env "msg_arg_data_size")
           (fun env ->
            IC.system_call env "msg_arg_data_copy")
           (fun env -> compile_unboxed_const 0L)))
    in
    Func.define_built_in env reply_name ["env", I64Type] [] (fun env ->
        callback_start env ^^
        (* Look up continuation *)
        let (set_closure, get_closure) = new_local env "closure" in
        G.i (LocalGet (nr 0l)) ^^
        ContinuationTable.recall env ^^
        Arr.load_field env 0L ^^ (* get the reply closure *)
        set_closure ^^
        get_closure ^^
        Closure.prepare_closure_call env ^^

        (* Deserialize/Blobify reply arguments  *)
        from_arg_data env ^^

        get_closure ^^
        Closure.call_closure env arity 0 ^^

        callback_cleanup env
      );

    let reject_name = "@reject_callback" in
    Func.define_built_in env reject_name ["env", I64Type] [] (fun env ->
        callback_start env ^^
        (* Look up continuation *)
        let (set_closure, get_closure) = new_local env "closure" in
        G.i (LocalGet (nr 0l)) ^^
        ContinuationTable.recall env ^^
        Arr.load_field env 1L ^^ (* get the reject closure *)
        set_closure ^^
        get_closure ^^
        Closure.prepare_closure_call env ^^
        (* Synthesize value of type `Text`, the error message
           (The error code is fetched via a prim)
        *)
        IC.error_value env ^^

        get_closure ^^
        Closure.call_closure env 1 0 ^^

        callback_cleanup env
      );

    (* result is a function that accepts a list of closure getters, from which
       the first and second must be the reply and reject continuations. *)
    fun closure_getters ->
      let set_cb_index, get_cb_index = new_local env "cb_index" in
      Arr.lit env Tagged.T closure_getters ^^
      ContinuationTable.remember env ^^
      set_cb_index ^^

      (* return arguments for the ic.call *)
      compile_unboxed_const (Wasm.I64_convert.extend_i32_u (E.add_fun_ptr env (E.built_in env reply_name))) ^^
      get_cb_index ^^
      compile_unboxed_const (Wasm.I64_convert.extend_i32_u (E.add_fun_ptr env (E.built_in env reject_name))) ^^
      get_cb_index

  let closures_to_reply_reject_callbacks env ts =
    closures_to_reply_reject_callbacks_aux env (Some ts)
  let closures_to_raw_reply_reject_callbacks env  =
    closures_to_reply_reject_callbacks_aux env None

  let ignoring_callback env =
    (* for one-way calls, we use an invalid table entry as the callback. this
       way, the callback, when it comes back, will (safely) trap, even if the
       module has completely changed in between. This way, one-way calls do not
       get in the way of safe instantaneous upgrades *)
    compile_unboxed_const 0xFFFF_FFFFL (* IC does not support -1 in 64-bit, thus use zero-extended 32-bit -1 *)

  let cleanup_callback env =
    let name = "@cleanup_callback" in
    Func.define_built_in env name ["env", I64Type] [] (fun env ->
        G.i (LocalGet (nr 0l)) ^^
        ContinuationTable.recall env ^^
        Arr.load_field env 2L ^^ (* get the cleanup closure *)
        let set_closure, get_closure = new_local env "closure" in
        set_closure ^^ get_closure ^^
        Closure.prepare_closure_call env ^^
        get_closure ^^
        Closure.call_closure env 0 0);
    compile_unboxed_const (Wasm.I64_convert.extend_i32_u (E.add_fun_ptr env (E.built_in env name)))

  let ic_call_threaded env purpose get_meth_pair push_continuations
    add_data add_cycles =
    match E.mode env with
    | Flags.ICMode
    | Flags.RefMode ->
      let message = Printf.sprintf "could not perform %s" purpose in
      let (set_cb_index, get_cb_index) = new_local env "cb_index" in
      (* The callee *)
      get_meth_pair ^^ Arr.load_field env 0L ^^ Blob.as_ptr_len env ^^
      (* The method name *)
      get_meth_pair ^^ Arr.load_field env 1L ^^ Blob.as_ptr_len env ^^
      (* The reply and reject callback *)
      push_continuations ^^
      set_cb_index ^^ get_cb_index ^^
      (* initiate call *)
      IC.system_call env "call_new" ^^
      cleanup_callback env ^^
      get_cb_index ^^
      IC.system_call env "call_on_cleanup" ^^
      (* the data *)
      add_data get_cb_index ^^
      IC.system_call env "call_data_append" ^^
      (* the cycles *)
      add_cycles ^^
      (* done! *)
      IC.system_call env "call_perform" ^^
      G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
      IC.set_call_perform_status env ^^
      Blob.lit env Tagged.T message ^^
      IC.set_call_perform_message env ^^
      IC.get_call_perform_status env ^^
      compile_unboxed_const 0L ^^
      compile_comparison I64Op.Ne ^^
      (* save error code, cleanup on error *)
      E.if0
      begin (* send failed *)
        if !Flags.trap_on_call_error then
          E.trap_with env message
        else
        (* Recall (don't leak) continuations *)
        get_cb_index ^^
        ContinuationTable.recall env ^^
        G.i Drop
      end
      begin (* send succeeded *)
        G.nop
      end
    | _ ->
      E.trap_with env (Printf.sprintf "cannot perform %s when running locally" purpose)

  let ic_call env ts1 ts2 get_meth_pair get_arg get_k get_r get_c =
    ic_call_threaded
      env
      "remote call"
      get_meth_pair
      (closures_to_reply_reject_callbacks env ts2 [get_k; get_r; get_c])
      (fun _ -> get_arg ^^ Serialization.serialize env ts1)

  let ic_call_raw env get_meth_pair get_arg get_k get_r get_c =
    ic_call_threaded
      env
      "raw call"
      get_meth_pair
      (closures_to_raw_reply_reject_callbacks env [get_k; get_r; get_c])
      (fun _ -> get_arg ^^ Blob.as_ptr_len env)

  let ic_self_call env ts get_meth_pair get_future get_k get_r get_c =
    ic_call_threaded
      env
      "self call"
      get_meth_pair
      (* Storing the tuple away, future_array_index = 3, keep in sync with rts/continuation_table.rs *)
      (closures_to_reply_reject_callbacks env ts [get_k; get_r; get_c; get_future])
      (fun get_cb_index ->
        get_cb_index ^^
        TaggedSmallWord.msb_adjust Type.Nat32 ^^
        Serialization.serialize env Type.[nat32])

  let ic_call_one_shot env ts get_meth_pair get_arg add_cycles =
    match E.mode env with
    | Flags.ICMode
    | Flags.RefMode ->
      (* The callee *)
      get_meth_pair ^^ Arr.load_field env 0L ^^ Blob.as_ptr_len env ^^
      (* The method name *)
      get_meth_pair ^^ Arr.load_field env 1L ^^ Blob.as_ptr_len env ^^
      (* The reply callback *)
      ignoring_callback env ^^
      compile_unboxed_const 0L ^^
      (* The reject callback *)
      ignoring_callback env ^^
      compile_unboxed_const 0L ^^
      IC.system_call env "call_new" ^^
      (* the data *)
      get_arg ^^ Serialization.serialize env ts ^^
      IC.system_call env "call_data_append" ^^
      (* the cycles *)
      add_cycles ^^
      IC.system_call env "call_perform" ^^
      G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
      (* This is a one-shot function: just remember error code *)
      (if !Flags.trap_on_call_error then
         (* legacy: discard status, proceed as if all well *)
         G.i Drop ^^
         compile_unboxed_zero ^^
         IC.set_call_perform_status env ^^
         Blob.lit env Tagged.T "" ^^
         IC.set_call_perform_message env
       else
         IC.set_call_perform_status env ^^
         Blob.lit env Tagged.T "could not perform oneway" ^^
         IC.set_call_perform_message env)

    | _ -> assert false

  let equate_msgref env =
    let (set_meth_pair1, get_meth_pair1) = new_local env "meth_pair1" in
    let (set_meth_pair2, get_meth_pair2) = new_local env "meth_pair2" in
    set_meth_pair2 ^^ set_meth_pair1 ^^
    get_meth_pair1 ^^ Arr.load_field env 0L ^^
    get_meth_pair2 ^^ Arr.load_field env 0L ^^
    Blob.compare env (Some Operator.EqOp) ^^
    E.if1 I64Type
    begin
      get_meth_pair1 ^^ Arr.load_field env 1L ^^
      get_meth_pair2 ^^ Arr.load_field env 1L ^^
      Blob.compare env (Some Operator.EqOp)
    end
    begin
      Bool.lit false
    end

  let export_async_method env =
    let name = IC.async_method_name in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        let (set_closure, get_closure) = new_local env "closure" in

        message_start env (Type.Shared Type.Write) ^^

        (* Check that we are calling this *)
        IC.assert_caller_self env ^^

        (* Deserialize and look up continuation argument *)
        Serialization.deserialize env Type.[nat32] ^^
        TaggedSmallWord.lsb_adjust Type.Nat32 ^^
        ContinuationTable.peek_future env ^^
        set_closure ^^
        get_closure ^^
        Closure.prepare_closure_call env ^^
        get_closure ^^
        Closure.call_closure env 0 0 ^^
        message_cleanup env (Type.Shared Type.Write)
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let export_gc_trigger_method env =
    let name = IC.gc_trigger_method_name in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        (* THe GC trigger is also blocked during incremental (de)stabilization.
           This is checked in `Lifecycle.trans` being called by `message_start` *)
        message_start env (Type.Shared Type.Write) ^^
        (* Check that we are called from this or a controller, w/o allocation *)
        IC.assert_caller_self_or_controller env ^^
        (* To avoid more failing allocation, don't deserialize args nor serialize reply,
           i.e. don't even try to do this:
        Serialization.deserialize env [] ^^
        Tuple.compile_unit env ^^
        Serialization.serialize env [] ^^
        *)
        (* Instead, just ignore the argument and
           send a *statically* allocated, nullary reply *)
        IC.static_nullary_reply env ^^
        (* Finally, act like
        message_cleanup env (Type.Shared Type.Write)
           but *force* collection *)
        GC.record_mutator_instructions env ^^
        E.collect_garbage env true ^^
        GC.record_collector_instructions env ^^
        Lifecycle.trans env Lifecycle.Idle
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let export_stabilization_limits env =
    let moc_stabilization_instruction_limit_fi =
      E.add_fun env "moc_stabilization_instruction_limit" (
        Func.of_body env [] [I64Type] (fun env ->
          (* To use the instruction budget well during upgrade,
             offer the entire upgrade instruction limit for the destabilization,
             since the stabilization can also be run before the upgrade. *)
          Lifecycle.during_explicit_upgrade env ^^
          E.if1 I64Type
            (compile_unboxed_const (Flags.(!stabilization_instruction_limit.update_call)))
            (compile_unboxed_const (Flags.(!stabilization_instruction_limit.upgrade)))
        )
      ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stabilization_instruction_limit";
      edesc = nr (FuncExport (nr moc_stabilization_instruction_limit_fi))
    });
    let moc_stable_memory_access_limit_fi =
      E.add_fun env "moc_stable_memory_access_limit" (
        Func.of_body env [] [I64Type] (fun env ->
          Lifecycle.during_explicit_upgrade env ^^
          E.if1 I64Type
            (compile_unboxed_const (Flags.(!stable_memory_access_limit.update_call)))
            (compile_unboxed_const (Flags.(!stable_memory_access_limit.upgrade)))
        )
      ) in
    E.add_export env (nr {
      name = Lib.Utf8.decode "moc_stable_memory_access_limit";
      edesc = nr (FuncExport (nr moc_stable_memory_access_limit_fi))
    })

end (* FuncDec *)

module IncrementalGraphStabilization = struct
  let register_globals env =
    E.add_global64 env "__stabilization_completed" Mutable 0L;
    E.add_global64 env "__destabilized_actor" Mutable 0L

  let is_stabilization_completed env =
    G.i (GlobalGet (nr (E.get_global env "__stabilization_completed")))
  let set_stabilization_completed env =
    G.i (GlobalSet (nr (E.get_global env "__stabilization_completed")))

  let get_destabilized_actor env =
    G.i (GlobalGet (nr (E.get_global env "__destabilized_actor")))
  let set_destabilized_actor env =
    G.i (GlobalSet (nr (E.get_global env "__destabilized_actor")))

  let async_stabilization_method_name = "@motoko_async_stabilization"

  let async_stabilization_reply_callback_name = "@async_stabilization_reply_callback"
  let async_stabilization_reply_callback env =
    Int64.of_int32 (E.add_fun_ptr env (E.built_in env async_stabilization_reply_callback_name))

  let async_stabilization_reject_callback_name = "@async_stabilization_reject_callback"
  let async_stabilization_reject_callback env =
    Int64.of_int32 (E.add_fun_ptr env (E.built_in env async_stabilization_reject_callback_name))

  let call_async_stabilization env =
    IC.get_self_reference env ^^ Blob.as_ptr_len env ^^
    Blob.lit_ptr_len env Tagged.T async_stabilization_method_name ^^
    compile_unboxed_const (async_stabilization_reply_callback env) ^^ compile_unboxed_const 0L ^^
    compile_unboxed_const (async_stabilization_reject_callback env) ^^ compile_unboxed_const 0L ^^
    IC.system_call env "call_new" ^^
    IC.system_call env "call_perform" ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    E.then_trap_with env "Async stabilization increment call failed"

  let define_async_stabilization_reply_callback env =
    Func.define_built_in env async_stabilization_reply_callback_name ["env", I64Type] [] (fun env ->
      is_stabilization_completed env ^^
      E.if0
        begin
          (* Sucessful completion of the async stabilization sequence. *)
          IC.static_nullary_reply env
          (* Skip garbage collection. *)
          (* Stay in lifecycle state `InStabilization`. *)
        end
        begin
          (* Trigger next async stabilization increment. *)
          call_async_stabilization env
        end)

  let define_async_stabilization_reject_callback env =
    Func.define_built_in env async_stabilization_reject_callback_name ["env", I64Type] [] (fun env ->
      IC.error_message env ^^
      Blob.as_ptr_len env ^^
      IC.system_call env "msg_reject")

  let export_async_stabilization_method env =
    let name = async_stabilization_method_name in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        IC.assert_caller_self_or_controller env ^^
        (* All messages are blocked except this method and the upgrade. *)
        Lifecycle.trans env Lifecycle.InStabilization ^^
        (* Skip argument deserialization to avoid allocations. *)
        GraphCopyStabilization.graph_stabilization_increment env ^^
        set_stabilization_completed env ^^
        IC.static_nullary_reply env
        (* Skip garbage collection. *)
        (* Stay in lifecycle state `InStabilization`. *)
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let start_graph_stabilization env actor_type =
    GraphCopyStabilization.is_graph_stabilization_started env ^^
    (E.if0
      G.nop
      begin
        (* Extra safety measure stopping the GC during incremental stabilization,
           although it should not be called in lifecycle state `InStabilization`. *)
        E.call_import env "rts" "stop_gc_before_stabilization" ^^
        IC.get_actor_to_persist env ^^
        GraphCopyStabilization.start_graph_stabilization env actor_type
      end)

  let export_stabilize_before_upgrade_method env actor_type =
    let name = "__motoko_stabilize_before_upgrade" in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        IC.assert_caller_self_or_controller env ^^
        (* All messages are blocked except this method and the upgrade. *)
        Lifecycle.trans env Lifecycle.InStabilization ^^
        start_graph_stabilization env actor_type ^^
        call_async_stabilization env
        (* Stay in lifecycle state `InStabilization`. *)
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let complete_stabilization_on_upgrade env actor_type =
    start_graph_stabilization env actor_type ^^
    G.loop0
    begin
      GraphCopyStabilization.graph_stabilization_increment env ^^
      E.if0
        G.nop
        (G.i (Br (nr 1l)))
    end

  let async_destabilization_method_name = "@motoko_async_destabilization"

  let async_destabilization_reply_callback_name = "@async_destabilization_reply_callback"
  let async_destabilization_reply_callback env =
    Int64.of_int32 (E.add_fun_ptr env (E.built_in env async_destabilization_reply_callback_name))

  let async_destabilization_reject_callback_name = "@async_destabilization_reject_callback"
  let async_destabilization_reject_callback env =
    Int64.of_int32 (E.add_fun_ptr env (E.built_in env async_destabilization_reject_callback_name))

  let call_async_destabilization env =
    IC.get_self_reference env ^^ Blob.as_ptr_len env ^^
    Blob.lit_ptr_len env Tagged.T async_destabilization_method_name ^^
    compile_unboxed_const (async_destabilization_reply_callback env) ^^ compile_unboxed_const 0L ^^
    compile_unboxed_const (async_destabilization_reject_callback env) ^^ compile_unboxed_const 0L ^^
    IC.system_call env "call_new" ^^
    IC.system_call env "call_perform" ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    E.then_trap_with env "Async destabilization increment call failed"

  let complete_graph_destabilization env =
    IC.initialize_main_actor env ^^
    (* Allow other messages and allow garbage collection. *)
    E.call_import env "rts" "start_gc_after_destabilization" ^^
    Lifecycle.trans env Lifecycle.Idle

  let define_async_destabilization_reply_callback env =
    Func.define_built_in env async_destabilization_reply_callback_name ["env", I64Type] [] (fun env ->
      get_destabilized_actor env ^^
      compile_test I64Op.Eqz ^^
      E.if0
        begin
          (* Trigger next async destabilization increment. *)
          call_async_destabilization env
        end
        begin
          (* Send static reply of sucessful async destabilization sequence. *)
          IC.static_nullary_reply env
          (* Stay in lifecycle state `InDestabilization`. *)
        end)

  let define_async_destabilization_reject_callback env =
    Func.define_built_in env async_destabilization_reject_callback_name ["env", I64Type] [] (fun env ->
      IC.error_message env ^^
      Blob.as_ptr_len env ^^
      IC.system_call env "msg_reject")

  let destabilization_increment env actor_type =
    get_destabilized_actor env ^^
    compile_test I64Op.Eqz ^^
    (E.if0
      begin
        GraphCopyStabilization.graph_destabilization_increment env ^^
        (E.if0
          begin
            GraphCopyStabilization.get_graph_destabilized_actor env actor_type ^^
            set_destabilized_actor env ^^
            complete_graph_destabilization env
          end
          G.nop)
      end
      G.nop)

  let export_async_destabilization_method env actor_type =
    let name = async_destabilization_method_name in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        IC.assert_caller_self_or_controller env ^^
        (* Stay in lifecycle state `InDestabilization` if not yet completed. *)
        destabilization_increment env actor_type ^^
        IC.static_nullary_reply env
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let partial_destabilization_on_upgrade env actor_type =
    (* TODO: Verify that the post_upgrade hook cannot be directly called by the IC *)
    (* Garbage collection is disabled in `start_graph_destabilization` until destabilization has completed. *)
    GraphCopyStabilization.start_graph_destabilization env actor_type.Ir.pre ^^
    get_destabilized_actor env ^^
    compile_test I64Op.Eqz ^^
    E.if0
      begin
        destabilization_increment env actor_type.Ir.pre ^^
        get_destabilized_actor env ^^
        (E.if0
          G.nop
          begin
            (* All messages remain blocked except this method. *)
            Lifecycle.trans env Lifecycle.InDestabilization
            (* Since the canister initialization cannot perform async calls, the destabilization
               needs to be explicitly continued by calling `__motoko_destabilize_after_upgrade`. *)
          end)
      end
      G.nop

  let export_destabilize_after_upgrade_method env =
    let name = "__motoko_destabilize_after_upgrade" in
    begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      Func.define_built_in env name [] [] (fun env ->
        (* All messages are blocked except this method. *)
        IC.assert_caller_self_or_controller env ^^
        (* Skip argument deserialization to avoid allocations. *)
        call_async_destabilization env
        (* Stay in lifecycle state `InDestabilization`. *)
      );

      let fi = E.built_in env name in
      E.add_export env (nr {
        name = Lib.Utf8.decode ("canister_update " ^ name);
        edesc = nr (FuncExport (nr fi))
      })
    | _ -> ()
    end

  let load env =
    get_destabilized_actor env ^^
    compile_test I64Op.Eqz ^^
    E.then_trap_with env "Destabilization is not yet completed: Call __motoko_destabilize_after_upgrade" ^^
    get_destabilized_actor env
    (* Upgrade costs are already record in RTS for graph-copy-based (de-)stabilization. *)

  let define_methods env (actor_type : Ir.stable_actor_typ) =
    define_async_stabilization_reply_callback env;
    define_async_stabilization_reject_callback env;
    export_async_stabilization_method env;
    export_stabilize_before_upgrade_method env actor_type.Ir.post;
    define_async_destabilization_reply_callback env;
    define_async_destabilization_reject_callback env;
    export_async_destabilization_method env actor_type.Ir.pre;
    export_destabilize_after_upgrade_method env;

end (* IncrementalGraphStabilization *)

module Persistence = struct
  (* Stable memory version at the time of the canister upgrade or initialization.
     This version can be different to `StableMem.get_version` because the upgrade logic
     may update the stable memory version, e.g. lift to enhanced orthogonal persistence. *)
  let register_globals env =
    E.add_global64 env "__persistence_version" Mutable 0L;
    E.add_global64 env "__init_message_payload" Mutable 0L

  let get_persistence_version env =
    G.i (GlobalGet (nr (E.get_global env "__persistence_version")))
  let set_persistence_version env =
    G.i (GlobalSet (nr (E.get_global env "__persistence_version")))

  (* No GC running during destabilization while this global blob reference is used. *)
  let get_init_message_payload env =
    G.i (GlobalGet (nr (E.get_global env "__init_message_payload")))
  let set_init_message_payload env =
    G.i (GlobalSet (nr (E.get_global env "__init_message_payload")))

  let use_candid_destabilization env =
    get_persistence_version env ^^
    compile_unboxed_const StableMem.legacy_version_regions ^^ (* Version 0 to 2 *)
    compile_comparison I64Op.LeU

  let use_graph_destabilization env =
    get_persistence_version env ^^
    compile_eq_const StableMem.version_graph_copy_no_regions ^^
    get_persistence_version env ^^
    compile_eq_const StableMem.version_graph_copy_regions ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or))

  let use_enhanced_orthogonal_persistence env =
    get_persistence_version env ^^
    compile_eq_const StableMem.version_stable_heap_no_regions ^^
    get_persistence_version env ^^
    compile_eq_const StableMem.version_stable_heap_regions ^^
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Or))

  let initialize env actor_type =
    E.call_import env "rts" "read_persistence_version" ^^
    set_persistence_version env ^^
    use_graph_destabilization env ^^
    E.if0
      begin
        IncrementalGraphStabilization.partial_destabilization_on_upgrade env actor_type
        (* Potentially stay in lifecycle state `InDestabilization` *)
      end
      begin
        IC.initialize_main_actor env ^^
        Lifecycle.trans env Lifecycle.Idle
      end

  let load env actor_type =
    use_enhanced_orthogonal_persistence env ^^
    (E.if1 I64Type
      (EnhancedOrthogonalPersistence.load env actor_type)
      begin
        use_graph_destabilization env ^^
        E.if1 I64Type
          begin
            IncrementalGraphStabilization.load env ^^
            NewStableMemory.upgrade_version_from_graph_stabilization env ^^
            EnhancedOrthogonalPersistence.initialize env actor_type
          end
          begin
            use_candid_destabilization env ^^
            E.else_trap_with env "Unsupported persistence version. Use newer Motoko compiler version." ^^
            if not (!Flags.explicit_enhanced_orthogonal_persistence) then
              E.trap_with env "Detected implicit upgrade from classical orthogonal persistence to enhanced orthogonal persistence. Recompile with explicit flag --enhanced-orthogonal-persistence and redeploy to enable this irreversible migration."
            else G.nop ^^
            OldStabilization.load env actor_type (NewStableMemory.upgrade_version_from_candid env) ^^
            EnhancedOrthogonalPersistence.initialize env actor_type
          end
      end) ^^
    StableMem.region_init env

  let in_upgrade env =
    use_enhanced_orthogonal_persistence env ^^
    (E.if1 I64Type
      begin
       EnhancedOrthogonalPersistence.has_stable_actor env ^^
       Bool.from_rts_int32
      end
      begin
        use_graph_destabilization env ^^
        E.if1 I64Type
          begin
            Bool.lit true
          end
          begin
            use_candid_destabilization env ^^
            E.else_trap_with env "Unsupported persistence version. Use newer Motoko compiler version." ^^
            StableMem.stable64_size env ^^
            Bool.from_int64
          end
      end)

  let save env actor_type =
    GraphCopyStabilization.is_graph_stabilization_started env ^^
    E.if0
      (IncrementalGraphStabilization.complete_stabilization_on_upgrade env actor_type)
      (EnhancedOrthogonalPersistence.save env actor_type)
end (* Persistence *)

module PatCode = struct
  (* Pattern failure code on demand.

  Patterns in general can fail, so we want a block around them with a
  jump-label for the fail case. But many patterns cannot fail, in particular
  function arguments that are simple variables. In these cases, we do not want
  to create the block and the (unused) jump label. So we first generate the
  code, either as plain code (CannotFail) or as code with hole for code to run
  in case of failure (CanFail).
  *)

  type patternCode =
    | CannotFail of G.t
    | CanFail of (G.t -> G.t)

  let definiteFail = CanFail (fun fail -> fail)

  let (^^^) : patternCode -> patternCode -> patternCode = function
    | CannotFail is1 ->
      begin function
      | CannotFail is2 -> CannotFail (is1 ^^ is2)
      | CanFail is2 -> CanFail (fun k -> is1 ^^ is2 k)
      end
    | CanFail is1 ->
      begin function
      | CannotFail is2 -> CanFail (fun k ->  is1 k ^^ is2)
      | CanFail is2 -> CanFail (fun k -> is1 k ^^ is2 k)
      end

  let with_fail (fail_code : G.t) : patternCode -> G.t = function
    | CannotFail is -> is
    | CanFail is -> is fail_code

  let orElse : patternCode -> patternCode -> patternCode = function
    | CannotFail is1 -> fun _ -> CannotFail is1
    | CanFail is1 -> function
      | CanFail is2 -> CanFail (fun fail_code ->
          let inner_fail = G.new_depth_label () in
          let inner_fail_code = Bool.lit false ^^ G.branch_to_ inner_fail in
          G.labeled_block1 I64Type inner_fail (is1 inner_fail_code ^^ Bool.lit true) ^^
          E.if0 G.nop (is2 fail_code)
        )
      | CannotFail is2 -> CannotFail (
          let inner_fail = G.new_depth_label () in
          let inner_fail_code = Bool.lit false ^^ G.branch_to_ inner_fail in
          G.labeled_block1 I64Type inner_fail (is1 inner_fail_code ^^ Bool.lit true) ^^
          E.if0 G.nop is2
        )

  let orElses : patternCode list -> patternCode -> patternCode =
    List.fold_right orElse

  let patternFailTrap env = E.trap_with env "pattern failed"

  let orPatternFailure env pcode =
    with_fail (patternFailTrap env) pcode

  let orsPatternFailure env pcodes =
    orPatternFailure env (orElses pcodes definiteFail)

  let with_region at = function
    | CannotFail is -> CannotFail (G.with_region at is)
    | CanFail is -> CanFail (fun k -> G.with_region at (is k))

end (* PatCode *)
open PatCode

(* All the code above is independent of the IR *)
open Ir

module AllocHow = struct
  (*
  When compiling a (recursive) block, we need to do a dependency analysis, to
  find out how the things are allocated. The options are:
  - const:  completely known, constant, not stored anywhere (think static function)
            (no need to mention in a closure)
  - local:  only needed locally, stored in a Wasm local, immutable
            (can be copied into a closure by value)
  - local mutable: only needed locally, stored in a Wasm local, mutable
            (cannot be copied into a closure)
  - heap allocated: stored on the dynamic heap, address in Wasm local
            (can be copied into a closure by reference)
  - static heap: stored on the static heap, address known statically
            (no need to mention in a closure)

  The goal is to avoid dynamic allocation where possible (and use locals), and
  to avoid turning function references into closures.

  The rules are:
  - functions are const, unless they capture something that is not a const
    function or a static heap allocation.
    in particular, top-level functions are always const
  - everything that is captured on the top-level needs to be statically
    heap-allocated
  - everything that is captured before it is defined, or is captured and mutable
    needs to be dynamically heap-allocated
  - the rest can be local
  *)

  module M = Freevars.M
  module S = Freevars.S

  (*
  We represent this as a lattice as follows:
  *)
  type how = Const | LocalImmut of SR.t | LocalMut of SR.t | StoreHeap | StoreStatic
  type allocHow = how M.t

  let disjoint_union : allocHow -> allocHow -> allocHow =
    M.union (fun v _ _ -> fatal "AllocHow.disjoint_union: %s" v)

  let join : allocHow -> allocHow -> allocHow =
    M.union (fun _ x y -> Some (match x, y with
      | StoreStatic, StoreHeap | StoreHeap, StoreStatic
      ->  fatal "AllocHow.join: cannot join StoreStatic and StoreHeap"

      | _, StoreHeap     | StoreHeap,      _ -> StoreHeap
      | _, StoreStatic   | StoreStatic,    _ -> StoreStatic
      | _, LocalMut sr   | LocalMut sr,    _ -> LocalMut sr
      | _, LocalImmut sr | LocalImmut sr,  _ -> LocalImmut sr

      | Const, Const -> Const
    ))
  let joins = List.fold_left join M.empty

  let map_of_set = Freevars.map_of_set
  let set_of_map = Freevars.set_of_map

  (* Various filters used in the set operations below *)
  let is_local_mut _ = function
    | LocalMut _ -> true
    | _ -> false

  let is_local _ = function
    | LocalImmut _ | LocalMut _ -> true
    | _ -> false

  let how_captured lvl how seen captured =
    (* What to do so that we can capture something?
       * For local blocks, put on the dynamic heap:
         - mutable things
         - not yet defined things
       * For top-level blocks, put on the static heap:
         - everything that is non-static (i.e. still in locals)
    *)
    match lvl with
    | VarEnv.NotTopLvl ->
      map_of_set StoreHeap (S.union
        (S.inter (set_of_map (M.filter is_local_mut how)) captured)
        (S.inter (set_of_map (M.filter is_local how)) (S.diff captured seen))
      )
    | VarEnv.TopLvl ->
      map_of_set StoreStatic
        (S.inter (set_of_map (M.filter is_local how)) captured)

  (* A bit like StackRep.of_type, but only for those types and stackreps that
     we support in local variables *)
  let stackrep_of_type t =
    let open Type in
    match normalize t with
    | Prim ((Nat64 | Int64 | Nat32 | Int32 | Nat16 | Int16 | Nat8 | Int8 | Char) as pty) ->
       SR.UnboxedWord64 pty
    | Prim Float -> SR.UnboxedFloat64
    | _ -> SR.Vanilla

  let dec lvl how_outer (seen, how0) dec =
    let how_all = disjoint_union how_outer how0 in

    let (f,d) = Freevars.dec dec in
    let captured = S.inter (set_of_map how0) (Freevars.captured_vars f) in

    (* Which allocation is required for the things defined here? *)
    let how1 = match dec.it with
      (* Mutable variables are, well, mutable *)
      | VarD _ ->
        M.map (fun t -> LocalMut (stackrep_of_type t)) d

      (* Constant expressions (trusting static_vals.ml) *)
      | LetD (_, e) when e.note.Note.const ->
        M.map (fun _ -> (Const : how)) d

      (* References to mutboxes *)
      | RefD _ ->
        M.map (fun _ -> StoreHeap) d

      (* Everything else needs at least a local *)
      | _ ->
        M.map (fun t -> LocalImmut (stackrep_of_type t)) d in

    (* Which allocation does this require for its captured things? *)
    let how2 = how_captured lvl how_all seen captured in

    let how = joins [how0; how1; how2] in
    let seen' = S.union seen (set_of_map d)
    in (seen', how)

  (* find the allocHow for the variables currently in scope *)
  (* we assume things are mutable, as we do not know better here *)
  let how_of_ae ae : allocHow =
    M.map (fun (l, _) -> match l with
    | VarEnv.Const _        -> (Const : how)
    | VarEnv.Static _       -> StoreStatic
    | VarEnv.HeapInd _      -> StoreHeap
    | VarEnv.Local (sr, _)  -> LocalMut sr (* conservatively assume mutable *)
    | VarEnv.PublicMethod _ -> LocalMut SR.Vanilla
    ) ae.VarEnv.vars

  let decs (ae : VarEnv.t) decs captured_in_body : allocHow =
    let lvl = ae.VarEnv.lvl in
    let how_outer = how_of_ae ae in
    let defined_here = snd (Freevars.decs decs) in (* TODO: implement gather_decs more directly *)
    let how_outer = Freevars.diff how_outer defined_here in (* shadowing *)
    let how0 = M.map (fun _t -> (Const : how)) defined_here in
    let captured = S.inter (set_of_map defined_here) captured_in_body in
    let rec go how =
      let seen, how1 = List.fold_left (dec lvl how_outer) (S.empty, how) decs in
      assert (S.equal seen (set_of_map defined_here));
      let how2 = how_captured lvl how1 seen captured in
      let how' = join how1 how2 in
      if M.equal (=) how how' then how' else go how' in
    go how0

  (* Functions to extend the environment (and possibly allocate memory)
     based on how we want to store them. *)
  let add_local env ae how name typ : VarEnv.t * G.t =
    match M.find name how with
    | (Const : how) -> (ae, G.nop)
    | LocalImmut sr | LocalMut sr ->
      let ae1, _ = VarEnv.add_direct_local env ae name sr typ in
      (ae1, G.nop)
    | StoreHeap ->
      let ae1, i = VarEnv.add_local_with_heap_ind env ae name typ in
      let alloc_code = MutBox.alloc env ^^ G.i (LocalSet (nr i)) in
      (ae1, alloc_code)
    | StoreStatic ->
      let index = MutBox.add_global_mutbox env in
      let ae1 = VarEnv.add_static_variable ae name index typ in
      (ae1, G.nop)

  let add_local_for_alias env ae how name typ : VarEnv.t * G.t =
    match M.find name how with
    | StoreHeap ->
      let ae1, _ = VarEnv.add_local_with_heap_ind env ae name typ in
      ae1, G.nop
    | _ -> assert false

end (* AllocHow *)

module Cost = struct
  let call env =
    Func.share_code2 Func.Always env "cost_call"
      (("method_name_size", I64Type), ("payload_size", I64Type))
      [IC.i]
      (fun env get_method_name_size get_payload_size ->
        Stack.with_words env "dst" 2L (fun get_dst ->
          get_method_name_size ^^
          get_payload_size ^^
          get_dst ^^
          IC.ic_system_call "cost_call" env ^^
          get_dst ^^
          Cycles.from_word128_ptr env
        )
      )

  let create_canister env =
    Func.share_code0 Func.Always env "cost_create_canister" [I64Type] (fun env ->
      Stack.with_words env "dst" 2L (fun get_dst ->
        get_dst ^^
        IC.ic_system_call "cost_create_canister" env ^^
        get_dst ^^
        Cycles.from_word128_ptr env
      )
    )

  let http_request env =
    Func.share_code2 Func.Always env "cost_http_request"
      (("request_size", I64Type), ("max_res_bytes", I64Type))
      [IC.i]
      (fun env get_request_size get_max_res_bytes ->
        Stack.with_words env "dst" 2L (fun get_dst ->
          get_request_size ^^
          get_max_res_bytes ^^
          get_dst ^^
          IC.ic_system_call "cost_http_request" env ^^
          get_dst ^^
          Cycles.from_word128_ptr env
        )
      )

  let sign_with_ecdsa env =
    Func.share_code2 Func.Always env "cost_sign_with_ecdsa"
      (("key_name", IC.i), ("curve", I32Type))
      [IC.i; I64Type]
      (fun env get_key_name get_curve ->
        Stack.with_words env "dst" 2L (fun get_dst ->
          get_key_name ^^ Text.to_blob env ^^ Blob.as_ptr_len env ^^
          get_curve ^^
          get_dst ^^
          IC.ic_system_call "cost_sign_with_ecdsa" env ^^
          G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
          TaggedSmallWord.msb_adjust Type.Nat32 ^^
          StackRep.adjust env (SR.UnboxedWord64 Type.Nat32) SR.Vanilla ^^
          get_dst ^^
          Cycles.from_word128_ptr env
        )
      )

  let sign_with_schnorr env =
    Func.share_code2 Func.Always env "cost_sign_with_schnorr"
      (("key_name", IC.i), ("algorithm", I32Type))
      [IC.i; I64Type]
      (fun env get_key_name get_algorithm ->
        Stack.with_words env "dst" 2L (fun get_dst ->
          get_key_name ^^ Text.to_blob env ^^ Blob.as_ptr_len env ^^
          get_algorithm ^^
          get_dst ^^
          IC.ic_system_call "cost_sign_with_schnorr" env ^^
          G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
          TaggedSmallWord.msb_adjust Type.Nat32 ^^
          StackRep.adjust env (SR.UnboxedWord64 Type.Nat32) SR.Vanilla ^^
          get_dst ^^
          Cycles.from_word128_ptr env
        )
      )
end

(* The actual compiler code that looks at the AST *)

(* wraps a bigint in range [0…2^64-1] into range [-2^63…2^63-1] *)
let nat64_to_int64 n =
  let open Big_int in
  if ge_big_int n (power_int_positive_int 2 63)
  then sub_big_int n (power_int_positive_int 2 64)
  else n


let const_lit_of_lit : Ir.lit -> Const.lit = function
  | BoolLit b     -> Const.Bool b
  | IntLit n
  | NatLit n      -> Const.BigInt (Numerics.Nat.to_big_int n)
  | Int8Lit n     -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Int8 (Numerics.Int_8.to_int64 n))
  | Nat8Lit n     -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Nat8 (Numerics.Nat8.to_int64 n))
  | Int16Lit n    -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Int16 (Numerics.Int_16.to_int64 n))
  | Nat16Lit n    -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Nat16 (Numerics.Nat16.to_int64 n))
  | Int32Lit n    -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Int32 (Numerics.Int_32.to_int64 n))
  | Nat32Lit n    -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Nat32 (Numerics.Nat32.to_int64 n))
  | Int64Lit n    -> Const.Word64 (Type.Int64, (Big_int.int64_of_big_int (Numerics.Int_64.to_big_int n)))
  | Nat64Lit n    -> Const.Word64 (Type.Nat64, (Big_int.int64_of_big_int (nat64_to_int64 (Numerics.Nat64.to_big_int n))))
  | CharLit c     -> Const.Vanilla (TaggedSmallWord.vanilla_lit Type.Char (Int64.of_int c))
  | NullLit       -> Const.Null
  | TextLit t     -> Const.Text t
  | BlobLit t     -> Const.Blob t
  | FloatLit f    -> Const.Float64 f

let const_of_lit lit =
  Const.Lit (const_lit_of_lit lit)

let compile_lit lit =
  SR.Const (const_of_lit lit), G.nop

let compile_lit_as env sr_out lit =
  let sr_in, code = compile_lit lit in
  code ^^ StackRep.adjust env sr_in sr_out

(* helper, traps with message *)
let then_arithmetic_overflow env =
  E.then_trap_with env "arithmetic overflow"

(* The first returned StackRep is for the arguments (expected), the second for the results (produced) *)
let compile_unop env t op =
  let open Operator in
  match op, t with
  | _, Type.Non ->
    SR.Vanilla, SR.Unreachable, G.i Unreachable
  | NegOp, Type.(Prim Int) ->
    SR.Vanilla, SR.Vanilla,
    BigNum.compile_neg env
  | NegOp, Type.(Prim ((Int8 | Int16 | Int32 | Int64) as p)) ->
    StackRep.of_type t, StackRep.of_type t,
    Func.share_code1 Func.Never env (prim_fun_name p "neg_trap") ("n", I64Type) [I64Type] (fun env get_n ->
      get_n ^^
      compile_eq_const 0x8000_0000_0000_0000L ^^
      then_arithmetic_overflow env ^^
      compile_unboxed_zero ^^
      get_n ^^
      G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
    )
  | NegOp, Type.(Prim Float) ->
    SR.UnboxedFloat64, SR.UnboxedFloat64,
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Neg))
  | NotOp, Type.(Prim (Nat64|Int64 as p)) ->
     SR.UnboxedWord64 p, SR.UnboxedWord64 p,
     compile_xor_const (-1L)
  | NotOp, Type.(Prim (Nat8|Nat16|Nat32|Int8|Int16|Int32 as ty)) ->
     StackRep.of_type t, StackRep.of_type t,
     compile_unboxed_const (TaggedSmallWord.mask_of_type ty) ^^
     G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor))
  | _ ->
    todo "compile_unop"
      (Wasm.Sexpr.Node ("BinOp", [ Arrange_ops.unop op ]))
      (SR.Vanilla, SR.Unreachable, E.trap_with env "TODO: compile_unop")

(* Logarithmic helpers for deciding whether we can carry out operations in constant bitwidth *)

(* helper, traps with message *)
let else_arithmetic_overflow env =
  E.else_trap_with env "arithmetic overflow"

(* helpers to decide if Int64 arithmetic can be carried out on the fast path *)
let additiveInt64_shortcut fast env get_a get_b slow =
  get_a ^^ get_a ^^ compile_shl_const 1L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^ compile_shrU_const 63L ^^
  get_b ^^ get_b ^^ compile_shl_const 1L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^ compile_shrU_const 63L ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
  compile_test I64Op.Eqz ^^
  E.if1 I64Type
    (get_a ^^ get_b ^^ fast)
    slow

let mulInt64_shortcut fast env get_a get_b slow =
  get_a ^^ get_a ^^ compile_shl_const 1L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
  get_b ^^ get_b ^^ compile_shl_const 1L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
  compile_unboxed_const 65L ^^ compile_comparison I64Op.GeU ^^
  E.if1 I64Type
    (get_a ^^ get_b ^^ fast)
    slow

let powInt64_shortcut fast env get_a get_b slow =
  get_b ^^ compile_test I64Op.Eqz ^^
  E.if1 I64Type
    compile_unboxed_one (* ^0 *)
    begin (* ^(1+n) *)
      get_a ^^ compile_unboxed_const (-1L) ^^ compile_comparison I64Op.Eq ^^
      E.if1 I64Type
        begin (* -1 ** (1+exp) == if even (1+exp) then 1 else -1 *)
          get_b ^^ compile_unboxed_one ^^
          G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^ compile_test I64Op.Eqz ^^
          E.if1 I64Type
            compile_unboxed_one
            get_a
        end
        begin
          get_a ^^ compile_shrS_const 1L ^^
          compile_test I64Op.Eqz ^^
          E.if1 I64Type
            get_a (* {0,1}^(1+n) *)
            begin
              get_b ^^ compile_unboxed_const 64L ^^
              compile_comparison I64Op.GeU ^^ then_arithmetic_overflow env ^^
              get_a ^^ get_a ^^ compile_shl_const 1L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^
              G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^ compile_sub_const 63L ^^
              get_b ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^
              compile_unboxed_const (-63L) ^^ compile_comparison I64Op.GeS ^^
              E.if1 I64Type
                (get_a ^^ get_b ^^ fast)
                slow
            end
        end
    end


(* kernel for Int64 arithmetic, invokes estimator for fast path *)
let compile_Int64_kernel env name op shortcut =
  Func.share_code2 Func.Always env (prim_fun_name Type.Int64 name)
    (("a", I64Type), ("b", I64Type)) [I64Type]
    BigNum.(fun env get_a get_b ->
    shortcut
      env
      get_a
      get_b
      begin
        let (set_res, get_res) = new_local env "res" in
        get_a ^^ from_signed_word64 env ^^
        get_b ^^ from_signed_word64 env ^^
        op env ^^
        set_res ^^ get_res ^^
        fits_signed_bits env 64 ^^
        else_arithmetic_overflow env ^^
        get_res ^^ truncate_to_word64 env
      end)


(* helpers to decide if Nat64 arithmetic can be carried out on the fast path *)
let additiveNat64_shortcut fast env get_a get_b slow =
  get_a ^^ compile_shrU_const 62L ^^
  get_b ^^ compile_shrU_const 62L ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Or)) ^^
  compile_test I64Op.Eqz ^^
  E.if1 I64Type
    (get_a ^^ get_b ^^ fast)
    slow

let mulNat64_shortcut fast env get_a get_b slow =
  get_a ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
  get_b ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
  compile_unboxed_const 64L ^^ compile_comparison I64Op.GeU ^^
  E.if1 I64Type
    (get_a ^^ get_b ^^ fast)
    slow

let powNat64_shortcut fast env get_a get_b slow =
  get_b ^^ compile_test I64Op.Eqz ^^
  E.if1 I64Type
    compile_unboxed_one (* ^0 *)
    begin (* ^(1+n) *)
      get_a ^^ compile_shrU_const 1L ^^
      compile_test I64Op.Eqz ^^
      E.if1 I64Type
        get_a (* {0,1}^(1+n) *)
        begin
          get_b ^^ compile_unboxed_const 64L ^^ compile_comparison I64Op.GeU ^^ then_arithmetic_overflow env ^^
          get_a ^^ G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz)) ^^ compile_sub_const 64L ^^
          get_b ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^ compile_unboxed_const (-64L) ^^ compile_comparison I64Op.GeS ^^
          E.if1 I64Type
            (get_a ^^ get_b ^^ fast)
            slow
        end
    end


(* kernel for Nat64 arithmetic, invokes estimator for fast path *)
let compile_Nat64_kernel env name op shortcut =
  Func.share_code2 Func.Always env (prim_fun_name Type.Nat64 name)
    (("a", I64Type), ("b", I64Type)) [I64Type]
    BigNum.(fun env get_a get_b ->
    shortcut
      env
      get_a
      get_b
      begin
        let (set_res, get_res) = new_local env "res" in
        get_a ^^ from_word64 env ^^
        get_b ^^ from_word64 env ^^
        op env ^^
        set_res ^^ get_res ^^
        fits_unsigned_bits env 64 ^^
        else_arithmetic_overflow env ^^
        get_res ^^ truncate_to_word64 env
      end)


(* Compiling Int/Nat32 ops by conversion to/from i64. *)

(* helper, expects i64 on stack *)
let enforce_32_unsigned_bits env =
  compile_bitand_const 0xFFFF_FFFF_0000_0000L ^^
  compile_test I64Op.Eqz ^^
  else_arithmetic_overflow env

(* helper, expects two identical i64s on stack *)
let enforce_32_signed_bits env =
  compile_shl_const 1L ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^
  enforce_32_unsigned_bits env

(* TODO: Combine this with `compile_smallInt_kernel`, to support `Int32`, `Int16`, and `Int8` at once. *)
let compile_Int32_kernel env name op =
     Func.share_code2 Func.Always env (prim_fun_name Type.Int32 name)
       (("a", I64Type), ("b", I64Type)) [I64Type]
       (fun env get_a get_b ->
         let (set_res, get_res) = new_local env "res" in
         get_a ^^ compile_shrS_const 32L ^^
         get_b ^^ compile_shrS_const 32L ^^
         G.i (Binary (Wasm_exts.Values.I64 op)) ^^
         set_res ^^ get_res ^^ get_res ^^
         enforce_32_signed_bits env ^^
         get_res ^^ compile_shl_const 32L)

(* TODO: Combine this with `compile_smallInt_kernel`, to support `Nat32`, `Nat16`, and `Nat8` at once. *)
let compile_Nat32_kernel env name op =
     Func.share_code2 Func.Always env (prim_fun_name Type.Nat32 name)
       (("a", I64Type), ("b", I64Type)) [I64Type]
       (fun env get_a get_b ->
         let (set_res, get_res) = new_local env "res" in
         get_a ^^ compile_shrU_const 32L ^^
         get_b ^^ compile_shrU_const 32L ^^
         G.i (Binary (Wasm_exts.Values.I64 op)) ^^
         set_res ^^ get_res ^^
         enforce_32_unsigned_bits env ^^
         get_res ^^ compile_shl_const 32L)

(* Customisable kernels for 8/16bit arithmetic via 64 bits. *)
(* TODO: Include the support for 32bit which is now also compact on 64-bit.
   Eventually, `compile_Int32_kernel` and `compile_Nat32_kernel` can be removed. *)

(* helper, expects i64 on stack *)
let enforce_unsigned_bits env n =
  compile_bitand_const Int64.(shift_left minus_one n) ^^
  compile_test I64Op.Eqz ^^
  else_arithmetic_overflow env

let enforce_16_unsigned_bits env = enforce_unsigned_bits env 16

(* helper, expects two identical i64s on stack *)
let enforce_signed_bits env n =
  compile_shl_const 1L ^^
  G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor)) ^^
  enforce_unsigned_bits env n

let enforce_16_signed_bits env = enforce_signed_bits env 16

let compile_smallInt_kernel' env ty name op =
  Func.share_code2 Func.Always env (prim_fun_name ty name)
    (("a", I64Type), ("b", I64Type)) [I64Type]
    (fun env get_a get_b ->
      let (set_res, get_res) = new_local env "res" in
      get_a ^^ compile_shrS_const 48L ^^
      get_b ^^ compile_shrS_const 48L ^^
      op ^^
      set_res ^^ get_res ^^ get_res ^^
      enforce_16_signed_bits env ^^
      get_res ^^ compile_shl_const 48L)

let compile_smallInt_kernel env ty name op =
  compile_smallInt_kernel' env ty name (G.i (Binary (Wasm_exts.Values.I64 op)))

let compile_smallNat_kernel' env ty name op =
  Func.share_code2 Func.Always env (prim_fun_name ty name)
    (("a", I64Type), ("b", I64Type)) [I64Type]
    (fun env get_a get_b ->
      let (set_res, get_res) = new_local env "res" in
      get_a ^^ compile_shrU_const 48L ^^
      get_b ^^ compile_shrU_const 48L ^^
      op ^^
      set_res ^^ get_res ^^
      enforce_16_unsigned_bits env ^^
      get_res ^^ compile_shl_const 48L)

let compile_smallNat_kernel env ty name op =
  compile_smallNat_kernel' env ty name (G.i (Binary (Wasm_exts.Values.I64 op)))

(* The first returned StackRep is for the arguments (expected), the second for the results (produced) *)
let compile_binop env t op : SR.t * SR.t * G.t =
  if t = Type.Non then SR.Vanilla, SR.Unreachable, G.i Unreachable else
  StackRep.of_type t,
  StackRep.of_type t,
  Operator.(match t, op with
  | Type.(Prim (Nat | Int)),                  AddOp -> BigNum.compile_add env
  | Type.(Prim (Nat64|Int64)),                WAddOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
  | Type.(Prim Int64),                        AddOp ->
    compile_Int64_kernel env "add" BigNum.compile_add
      (additiveInt64_shortcut (G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))))
  | Type.(Prim Nat64),                        AddOp ->
    compile_Nat64_kernel env "add" BigNum.compile_add
      (additiveNat64_shortcut (G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))))
  | Type.(Prim Nat),                          SubOp -> BigNum.compile_unsigned_sub env
  | Type.(Prim Int),                          SubOp -> BigNum.compile_signed_sub env
  | Type.(Prim (Nat | Int)),                  MulOp -> BigNum.compile_mul env
  | Type.(Prim (Nat64|Int64)),                WMulOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))
  | Type.(Prim Int64),                        MulOp ->
    compile_Int64_kernel env "mul" BigNum.compile_mul
      (mulInt64_shortcut (G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))))
  | Type.(Prim Nat64),                        MulOp ->
    compile_Nat64_kernel env "mul" BigNum.compile_mul
      (mulNat64_shortcut (G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul))))
  | Type.(Prim Nat64),                        DivOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.DivU))
  | Type.(Prim Nat64) ,                       ModOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.RemU))
  | Type.(Prim Int64),                        DivOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.DivS))
  | Type.(Prim Int64) ,                       ModOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.RemS))
  | Type.(Prim Nat),                          DivOp -> BigNum.compile_unsigned_div env
  | Type.(Prim Nat),                          ModOp -> BigNum.compile_unsigned_rem env
  | Type.(Prim (Nat64|Int64)),                WSubOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
  | Type.(Prim Int64),                        SubOp ->
    compile_Int64_kernel env "sub" BigNum.compile_signed_sub
      (additiveInt64_shortcut (G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))))
  | Type.(Prim Nat64),                        SubOp ->
    compile_Nat64_kernel env "sub" BigNum.compile_unsigned_sub
      (fun env get_a get_b ->
        additiveNat64_shortcut
          (compile_comparison I64Op.GeU ^^
           else_arithmetic_overflow env ^^
           get_a ^^ get_b ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub)))
          env get_a get_b)
  | Type.(Prim Int),                          DivOp -> BigNum.compile_signed_div env
  | Type.(Prim Int),                          ModOp -> BigNum.compile_signed_mod env

  | Type.Prim Type.(Nat8|Nat16|Nat32|Int8|Int16|Int32),
                                              WAddOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Add))
  | Type.(Prim Int32),                        AddOp -> compile_Int32_kernel env "add" I64Op.Add
  | Type.Prim Type.(Int8 | Int16 as ty),      AddOp -> compile_smallInt_kernel env ty "add" I64Op.Add
  | Type.(Prim Nat32),                        AddOp -> compile_Nat32_kernel env "add" I64Op.Add
  | Type.Prim Type.(Nat8 | Nat16 as ty),      AddOp -> compile_smallNat_kernel env ty "add" I64Op.Add
  | Type.(Prim Float),                        AddOp -> G.i (Binary (Wasm_exts.Values.F64 F64Op.Add))
  | Type.Prim Type.(Nat8|Nat16|Nat32|Int8|Int16|Int32),
                                              WSubOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Sub))
  | Type.(Prim Int32),                        SubOp -> compile_Int32_kernel env "sub" I64Op.Sub
  | Type.(Prim (Int8|Int16 as ty)),           SubOp -> compile_smallInt_kernel env ty "sub" I64Op.Sub
  | Type.(Prim Nat32),                        SubOp -> compile_Nat32_kernel env "sub" I64Op.Sub
  | Type.(Prim (Nat8|Nat16 as ty)),           SubOp -> compile_smallNat_kernel env ty "sub" I64Op.Sub
  | Type.(Prim Float),                        SubOp -> G.i (Binary (Wasm_exts.Values.F64 F64Op.Sub))
  | Type.Prim Type.(Nat8|Nat16|Nat32|Int8|Int16|Int32 as ty),
                                              WMulOp -> TaggedSmallWord.compile_word_mul env ty
  | Type.(Prim Int32),                        MulOp -> compile_Int32_kernel env "mul" I64Op.Mul
  | Type.(Prim Int16),                        MulOp -> compile_smallInt_kernel env Type.Int16 "mul" I64Op.Mul
  | Type.(Prim Int8),                         MulOp -> compile_smallInt_kernel' env Type.Int8 "mul"
                                                         (compile_shrS_const 8L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)))
  | Type.(Prim Nat32),                        MulOp -> compile_Nat32_kernel env "mul" I64Op.Mul
  | Type.(Prim Nat16),                        MulOp -> compile_smallNat_kernel env Type.Nat16 "mul" I64Op.Mul
  | Type.(Prim Nat8),                         MulOp -> compile_smallNat_kernel' env Type.Nat8 "mul"
                                                         (compile_shrU_const 8L ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)))
  | Type.(Prim Float),                        MulOp -> G.i (Binary (Wasm_exts.Values.F64 F64Op.Mul))
  | Type.(Prim (Nat8|Nat16|Nat32 as ty)),     DivOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.DivU)) ^^
                                                       TaggedSmallWord.msb_adjust ty
  | Type.(Prim (Nat8|Nat16|Nat32)),           ModOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.RemU))
  | Type.(Prim (Int8|Int16|Int32 as ty)),           DivOp ->
    Func.share_code2 Func.Always env (prim_fun_name ty "div")
      (("a", I64Type), ("b", I64Type)) [I64Type]
      (fun env get_a get_b ->
        let (set_res, get_res) = new_local env "res" in
        get_a ^^ get_b ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.DivS)) ^^
        TaggedSmallWord.msb_adjust ty ^^ set_res ^^
        get_a ^^ compile_eq_const 0x8000_0000_0000_0000L ^^
        E.if_ env [I64Type]
          begin
            get_b ^^ TaggedSmallWord.lsb_adjust ty ^^ compile_eq_const (-1L) ^^
            E.if_ env [I64Type]
              (G.i Unreachable)
              get_res
          end
          get_res)
  | Type.(Prim Float),                        DivOp -> G.i (Binary (Wasm_exts.Values.F64 F64Op.Div))
  | Type.(Prim Float),                        ModOp -> E.call_import env "rts" "fmod"
  | Type.(Prim (Int8|Int16|Int32)),           ModOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.RemS))
  | Type.(Prim (Nat8|Nat16|Nat32 as ty)),     WPowOp -> TaggedSmallWord.compile_nat_power env ty
  | Type.(Prim (Int8|Int16|Int32 as ty)),     WPowOp -> TaggedSmallWord.compile_int_power env ty
  | Type.(Prim ((Nat8|Nat16|Nat32) as ty)),         PowOp ->
    Func.share_code2 Func.Always env (prim_fun_name ty "pow")
      (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        let (set_res, get_res) = new_local env "res" in
        let bits = TaggedSmallWord.bits_of_type ty in
        let set_n = G.setter_for get_n in
        let set_exp = G.setter_for get_exp in
        get_n ^^ TaggedSmallWord.lsb_adjust ty ^^ set_n ^^
        get_exp ^^ TaggedSmallWord.lsb_adjust ty ^^ set_exp ^^
        get_exp ^^ Bool.from_int64 ^^
        E.if1 I64Type
          begin
            get_n ^^ compile_shrU_const 1L ^^
            Bool.from_int64 ^^
            E.if1 I64Type
              begin
                let overflow_type = match ty with
                | Type.Nat32 -> Type.Nat64
                | Type.(Nat8 | Nat16) -> Type.Nat32
                | _ -> assert false in
                let overflow_type_bits = TaggedSmallWord.bits_of_type overflow_type in
                let overflow_boundary = -Int.(sub (mul overflow_type_bits 2) 2) in
                get_exp ^^ compile_unboxed_const 64L ^^
                compile_comparison I64Op.GeU ^^ then_arithmetic_overflow env ^^
                unsigned_dynamics get_n ^^ compile_sub_const (Int64.of_int bits) ^^
                get_exp ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^
                compile_unboxed_const (Int64.of_int overflow_boundary) ^^
                compile_comparison I64Op.LtS ^^ then_arithmetic_overflow env ^^
                get_n ^^ get_exp ^^
                TaggedSmallWord.compile_nat_power env Type.Nat64 ^^ set_res ^^
                get_res ^^ enforce_unsigned_bits env bits ^^
                get_res ^^ TaggedSmallWord.msb_adjust ty
              end
              (get_n ^^ TaggedSmallWord.msb_adjust ty) (* n@{0,1} ** (1+exp) == n *)
          end
          (compile_unboxed_one ^^ TaggedSmallWord.msb_adjust ty)) (* x ** 0 == 1 *)
  | Type.(Prim ((Int8|Int16|Int32) as ty)),         PowOp ->
    Func.share_code2 Func.Always env (prim_fun_name ty "pow")
      (("n", I64Type), ("exp", I64Type)) [I64Type]
      (fun env get_n get_exp ->
        let (set_res, get_res) = new_local env "res" in
        let bits = TaggedSmallWord.bits_of_type ty in
        let set_n = G.setter_for get_n in
        let set_exp = G.setter_for get_exp in
        get_n ^^ TaggedSmallWord.lsb_adjust ty ^^ set_n ^^
        get_exp ^^ TaggedSmallWord.lsb_adjust ty ^^ set_exp ^^
        get_exp ^^ compile_unboxed_zero ^^
        compile_comparison I64Op.LtS ^^ E.then_trap_with env "negative power" ^^
        get_exp ^^ Bool.from_int64 ^^
        E.if1 I64Type
          begin
            get_n ^^ compile_unboxed_one ^^ compile_comparison I64Op.LeS ^^
            get_n ^^ compile_unboxed_const (-1L) ^^ compile_comparison I64Op.GeS ^^
            G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
            E.if1 I64Type
              begin
                get_n ^^ compile_unboxed_zero ^^ compile_comparison I64Op.LtS ^^
                E.if1 I64Type
                  begin
                    (* -1 ** (1+exp) == if even (1+exp) then 1 else -1 *)
                    get_exp ^^ compile_unboxed_one ^^ G.i (Binary (Wasm_exts.Values.I64 I64Op.And)) ^^
                    E.if1 I64Type
                      (get_n ^^ TaggedSmallWord.msb_adjust ty)
                      (compile_unboxed_const
                        Int64.(shift_left one (to_int (TaggedSmallWord.shift_of_type ty))))
                  end
                  (get_n ^^ TaggedSmallWord.msb_adjust ty) (* n@{0,1} ** (1+exp) == n *)
              end
              begin
                let overflow_type = match ty with
                | Type.Int32 -> Type.Int64
                | Type.(Int8 | Int16) -> Type.Int32
                | _ -> assert false in
                let overflow_type_bits = TaggedSmallWord.bits_of_type overflow_type in
                let overflow_boundary = -Int.(sub (mul overflow_type_bits 2) 2) in
                get_exp ^^ compile_unboxed_const 64L ^^
                compile_comparison I64Op.GeU ^^ then_arithmetic_overflow env ^^
                signed_dynamics get_n ^^ compile_sub_const (Int64.of_int (Int.sub bits 1)) ^^
                get_exp ^^
                G.i (Binary (Wasm_exts.Values.I64 I64Op.Mul)) ^^
                compile_unboxed_const (Int64.of_int overflow_boundary) ^^
                compile_comparison I64Op.LtS ^^ then_arithmetic_overflow env ^^
                get_n ^^ get_exp ^^
                TaggedSmallWord.compile_nat_power env Type.Nat64 ^^ set_res ^^
                get_res ^^ get_res ^^ enforce_signed_bits env bits ^^
                get_res ^^ TaggedSmallWord.msb_adjust ty
              end
          end
          (compile_unboxed_one ^^ TaggedSmallWord.msb_adjust ty)) (* x ** 0 == 1 *)
  | Type.(Prim Int),                          PowOp ->
    let pow = BigNum.compile_unsigned_pow env in
    let (set_n, get_n) = new_local env "n" in
    let (set_exp, get_exp) = new_local env "exp" in
    set_exp ^^ set_n ^^
    get_exp ^^ BigNum.compile_is_negative env ^^
    E.then_trap_with env "negative power" ^^
    get_n ^^ get_exp ^^ pow
  | Type.(Prim Nat64),                        WPowOp -> Word64.compile_unsigned_pow env
  | Type.(Prim Int64),                        WPowOp -> Word64.compile_signed_wpow env
  | Type.(Prim Nat64),                        PowOp ->
    compile_Nat64_kernel env "pow"
      BigNum.compile_unsigned_pow
      (powNat64_shortcut (Word64.compile_unsigned_pow env))
  | Type.(Prim Int64),                        PowOp ->
    let (set_exp, get_exp) = new_local env "exp" in
    set_exp ^^ get_exp ^^
    compile_unboxed_const 0L ^^
    compile_comparison I64Op.LtS ^^
    E.then_trap_with env "negative power" ^^
    get_exp ^^
    compile_Int64_kernel
      env "pow" BigNum.compile_unsigned_pow
      (powInt64_shortcut (Word64.compile_unsigned_pow env))
  | Type.(Prim Nat),                          PowOp -> BigNum.compile_unsigned_pow env
  | Type.(Prim Float),                        PowOp -> E.call_import env "rts" "pow"
  | Type.(Prim (Nat8|Nat16|Nat32|Nat64|Int8|Int16|Int32|Int64)),
                                              AndOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.And))
  | Type.(Prim (Nat8|Nat16|Nat32|Nat64|Int8|Int16|Int32|Int64)),
                                              OrOp  -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Or))
  | Type.(Prim (Nat8|Nat16|Nat32|Nat64|Int8|Int16|Int32|Int64)),
                                              XorOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Xor))
  | Type.(Prim (Nat64|Int64)),                ShLOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl))
  | Type.(Prim (Nat8|Nat16|Nat32|Int8|Int16|Int32 as ty)),
                                              ShLOp -> TaggedSmallWord.(
     lsb_adjust ty ^^ clamp_shift_amount ty ^^
     G.i (Binary (Wasm_exts.Values.I64 I64Op.Shl)))
  | Type.(Prim Nat64),                        ShROp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrU))
  | Type.(Prim (Nat8|Nat16|Nat32 as ty)),     ShROp -> TaggedSmallWord.(
     lsb_adjust ty ^^ clamp_shift_amount ty ^^
     G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrU)) ^^
     sanitize_word_result ty)
  | Type.(Prim Int64),                        ShROp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrS))
  | Type.(Prim (Int8|Int16|Int32 as ty)),     ShROp -> TaggedSmallWord.(
     lsb_adjust ty ^^ clamp_shift_amount ty ^^
     G.i (Binary (Wasm_exts.Values.I64 I64Op.ShrS)) ^^
     sanitize_word_result ty)
  | Type.(Prim (Nat64|Int64)),                RotLOp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Rotl))
  | Type.(Prim (Nat8|Nat16|Nat32|Int8|Int16|Int32 as ty)),
                                              RotLOp -> TaggedSmallWord.rotl env ty
  | Type.(Prim (Nat64|Int64)),                RotROp -> G.i (Binary (Wasm_exts.Values.I64 I64Op.Rotr))
  | Type.(Prim (Nat8|Nat16|Nat32|Int8|Int16|Int32 as ty)),
                                              RotROp -> TaggedSmallWord.rotr env ty
  | Type.(Prim Text), CatOp -> Text.concat env
  | Type.Non, _ -> G.i Unreachable
  | _ -> todo_trap env "compile_binop" (Wasm.Sexpr.Node ("BinOp", [ Arrange_ops.binop op; Arrange_type.typ t]))
  )

let compile_eq env =
  let open Type in
  function
  | Prim Text -> Text.compare env Operator.EqOp
  | Prim (Blob|Principal) | Obj (Actor, _) -> Blob.compare env (Some Operator.EqOp)
  | Func (Shared _, _, _, _, _) -> FuncDec.equate_msgref env
  | Prim (Nat | Int) -> BigNum.compile_eq env
  | Prim (Bool | Int8 | Nat8 | Int16 | Nat16 | Int32 | Nat32 | Int64 | Nat64 | Char) ->
    compile_comparison I64Op.Eq
  | Non -> G.i Unreachable
  | Prim Float -> compile_comparison_f64 F64Op.Eq
  | t -> todo_trap env "compile_eq" (Arrange_type.typ t)

let get_relops = Operator.(function
  | GeOp -> Ge, I64Op.GeU, I64Op.GeS
  | GtOp -> Gt, I64Op.GtU, I64Op.GtS
  | LeOp -> Le, I64Op.LeU, I64Op.LeS
  | LtOp -> Lt, I64Op.LtU, I64Op.LtS
  | NeqOp -> assert false
  | _ -> failwith "uncovered relop")

let compile_comparison_op env t op =
  let bigintop, u64op, s64op = get_relops op in
  let open Type in
  match t with
    | Nat | Int -> BigNum.compile_relop env bigintop
    | Nat8 | Nat16 | Nat32 | Nat64 | Char -> compile_comparison u64op
    | Int8 | Int16 | Int32 | Int64 -> compile_comparison s64op
    | _ -> todo_trap env "compile_comparison" (Arrange_type.prim t)

let compile_relop env t op =
  if t = Type.Non then SR.Vanilla, G.i Unreachable else
  StackRep.of_type t,
  let open Operator in
  match t, op with
  | Type.(Prim Text), _ -> Text.compare env op
  | Type.(Prim (Blob|Principal)), _ -> Blob.compare env (Some op)
  | _, EqOp -> compile_eq env t
  | Type.(Prim (Nat | Nat8 | Nat16 | Nat32 | Nat64 | Int | Int8 | Int16 | Int32 | Int64 | Char as t1)), op1 ->
    compile_comparison_op env t1 op1
  | Type.(Prim Float), GtOp -> compile_comparison_f64 F64Op.Gt
  | Type.(Prim Float), GeOp -> compile_comparison_f64 F64Op.Ge
  | Type.(Prim Float), LeOp -> compile_comparison_f64 F64Op.Le
  | Type.(Prim Float), LtOp -> compile_comparison_f64 F64Op.Lt
  | _ -> todo_trap env "compile_relop" (Arrange_ops.relop op)

let compile_load_field env typ name =
  Object.load_idx env typ name


(* compile_lexp is used for expressions on the left of an assignment operator.
   Produces
   * preparation code, to run first
   * an expected stack rep
   * code that expects the value to be written in that stackrep, and consumes it
*)
let rec compile_lexp (env : E.t) ae lexp : G.t * SR.t * G.t =
  (fun (code, sr, fill_code) -> G.(with_region lexp.at code, sr, with_region lexp.at fill_code)) @@
  match lexp.it with
  | VarLE var -> Var.set_val env ae var
  | IdxLE (e1, e2) when potential_pointer (Arr.element_type env e1.note.Note.typ) ->
    compile_array_index env ae e1 e2 ^^
    compile_add_const ptr_unskew,
    SR.Vanilla,
    Tagged.write_with_barrier env
  | IdxLE (e1, e2) ->
    compile_array_index env ae e1 e2,
    SR.Vanilla,
    store_ptr
  | DotLE (e, n) when potential_pointer (Object.field_type env e.note.Note.typ n) ->
    compile_exp_vanilla env ae e ^^
    (* Only real objects have mutable fields, no need to branch on the tag *)
    Object.idx env e.note.Note.typ n ^^
    compile_add_const ptr_unskew,
    SR.Vanilla,
    Tagged.write_with_barrier env
  | DotLE (e, n) ->
    compile_exp_vanilla env ae e ^^
    (* Only real objects have mutable fields, no need to branch on the tag *)
    Object.idx env e.note.Note.typ n,
    SR.Vanilla,
    store_ptr

(* Common code for a[e] as lexp and as exp.
Traps or pushes the pointer to the element on the stack
*)
and compile_array_index env ae e1 e2 =
    compile_exp_vanilla env ae e1 ^^ (* offset to array payload *)
    compile_exp_vanilla env ae e2 ^^ (* idx *)
    Arr.idx_bigint env

and compile_prim_invocation (env : E.t) ae p es at =
  (* for more concise code when all arguments and result use the same sr *)
  let const_sr sr inst = sr, G.concat_map (compile_exp_as env ae sr) es ^^ inst in

  begin match p, es with
  (* Calls *)
  | CallPrim _, [e1; e2] ->
    let sort, control, _, arg_tys, ret_tys = Type.(as_func (promote e1.note.Note.typ)) in
    let n_args = List.length arg_tys in
    let return_arity = match control with
      | Type.Returns -> List.length ret_tys
      | Type.Replies -> 0
      | Type.Promises -> assert false in

    let fun_sr, code1 = compile_exp env ae e1 in

    (* we duplicate this pattern match to emulate pattern guards *)
    let call_as_prim = match fun_sr, sort with
      | SR.Const Const.Fun (_, mk_fi, Const.PrimWrapper prim), _ ->
         begin match n_args, e2.it with
         | 0, _ -> true
         | 1, _ -> true
         | n, PrimE (TupPrim, es) when List.length es = n -> true
         | _, _ -> false
         end
      | _ -> false in

    begin match fun_sr, sort with
      | SR.Const Const.Fun (_, mk_fi, Const.PrimWrapper prim), _ when call_as_prim ->
         assert (sort = Type.Local);
         (* Handle argument tuples *)
         begin match n_args, e2.it with
         | 0, _ ->
           let sr, code2 = compile_prim_invocation env ae prim [] at in
           sr,
           code1 ^^
           compile_exp_as env ae (StackRep.of_arity 0) e2 ^^
           code2
         | 1, _ ->
           compile_prim_invocation env ae prim [e2] at
         | n, PrimE (TupPrim, es) ->
           assert (List.length es = n);
           compile_prim_invocation env ae prim es at
         | _, _ ->
           (* ugly case; let's just call this as a function for now *)
           raise (Invalid_argument "call_as_prim was true?")
         end
      | SR.Const Const.Fun (_, mk_fi, _), _ ->
         assert (sort = Type.Local);
         StackRep.of_arity return_arity,

         code1 ^^
         compile_unboxed_zero ^^ (* A dummy closure *)
         compile_exp_as env ae (StackRep.of_arity n_args) e2 ^^ (* the args *)
         G.i (Call (nr (mk_fi()))) ^^
         FakeMultiVal.load env (Lib.List.make return_arity I64Type)
      | _, Type.Local ->
         let (set_clos, get_clos) = new_local env "clos" in

         StackRep.of_arity return_arity,
         code1 ^^ StackRep.adjust env fun_sr SR.Vanilla ^^
         set_clos ^^
         get_clos ^^
         Closure.prepare_closure_call env ^^
         compile_exp_as env ae (StackRep.of_arity n_args) e2 ^^
         get_clos ^^
         Closure.call_closure env n_args return_arity
      | _, Type.Shared _ ->
         (* Non-one-shot functions have been rewritten in async.ml *)
         assert (control = Type.Returns);

         let (set_meth_pair, get_meth_pair) = new_local env "meth_pair" in
         let (set_arg, get_arg) = new_local env "arg" in
         let _, _, _, ts, _ = Type.as_func e1.note.Note.typ in
         let add_cycles = Internals.add_cycles env ae in

         StackRep.of_arity return_arity,
         code1 ^^ StackRep.adjust env fun_sr SR.Vanilla ^^
         set_meth_pair ^^
         compile_exp_vanilla env ae e2 ^^ set_arg ^^

         FuncDec.ic_call_one_shot env ts get_meth_pair get_arg add_cycles
    end

  (* Operators *)
  | UnPrim (_, Operator.PosOp), [e1] -> compile_exp env ae e1
  | UnPrim (t, op), [e1] ->
    let sr_in, sr_out, code = compile_unop env t op in
    sr_out,
    compile_exp_as env ae sr_in e1 ^^
    code
  | BinPrim (t, op), [e1;e2] ->
    let sr_in, sr_out, code = compile_binop env t op in
    sr_out,
    compile_exp_as env ae sr_in e1 ^^
    compile_exp_as env ae sr_in e2 ^^
    code
  (* special case: recognize negation *)
  | RelPrim (Type.(Prim Bool), Operator.EqOp), [e1; {it = LitE (BoolLit false); _}] ->
    SR.bool,
    compile_exp_as_test env ae e1 ^^
    compile_test I64Op.Eqz
  | RelPrim (t, op), [e1;e2] ->
    let sr, code = compile_relop env t op in
    SR.bool,
    compile_exp_as env ae sr e1 ^^
    compile_exp_as env ae sr e2 ^^
    code

  (* Tuples *)
  | TupPrim, es ->
    SR.UnboxedTuple (List.length es),
    G.concat_map (compile_exp_vanilla env ae) es
  | ProjPrim n, [e1] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e1 ^^ (* offset to tuple (an array) *)
    Tuple.load_n env (Int64.of_int n)

  | OptPrim, [e] ->
    SR.Vanilla,
    Opt.inject env (compile_exp_vanilla env ae e)
  | TagPrim l, [e] ->
    SR.Vanilla,
    Variant.inject env l (compile_exp_vanilla env ae e)

  | DotPrim name, [e] ->
    let sr, code1 = compile_exp env ae e in
    begin match sr with
    | SR.Const Const.Obj fs ->
      let c = List.assoc name fs in
      SR.Const c, code1
    | _ ->
      SR.Vanilla,
      code1 ^^ StackRep.adjust env sr SR.Vanilla ^^
      Object.load_idx env e.note.Note.typ name
    end
  | ActorDotPrim name, [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    IC.actor_public_field env name

  | ArrayPrim (m, t), es ->
    SR.Vanilla,
    Arr.lit env Tagged.(if m = Ir.Var then M else I) (List.map (compile_exp_vanilla env ae) es)
  | IdxPrim, [e1; e2] ->
    SR.Vanilla,
    compile_array_index env ae e1 e2 ^^
    load_ptr
  (* NB: all these operations assume a valid array offset fits in a compact bignum *)
  | NextArrayOffset, [e] ->
    let one_untagged = Int64.shift_left 1L (64 - BitTagged.ubits_of Type.Int) in
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^ (* previous byte offset to array *)
    compile_add_const one_untagged (* preserving the tag in low bits *)
  | EqArrayOffset, [e1; e2] ->
    SR.bool,
    compile_exp_vanilla env ae e1 ^^
    BitTagged.sanity_check_tag __LINE__ env Type.Int ^^
    compile_exp_vanilla env ae e2 ^^
    BitTagged.sanity_check_tag __LINE__ env Type.Int ^^
    (* equate (without untagging) *)
    compile_comparison I64Op.Eq
  | DerefArrayOffset, [e1; e2] ->
    SR.Vanilla,
    (* NB: no bounds check on index *)
    compile_exp_vanilla env ae e1 ^^ (* skewed pointer to array *)
    Tagged.load_forwarding_pointer env ^^
    compile_exp_vanilla env ae e2 ^^ (* byte offset *)
    BitTagged.untag __LINE__ env Type.Int ^^
    (* TODO: Refactor 3L to use word_size *)
    compile_shl_const 3L ^^ (* effectively a multiplication by word_size *)
    (* Note: the below two lines compile to `i64.add; i64.load offset=OFFSET`
       with `OFFSET = Arr.header_size * word_size + ptr_unskew`,
       thus together also unskewing the pointer and skipping administrative
       fields, effectively arriving at the desired element *)
    G.i (Binary (Wasm_exts.Values.I64 I64Op.Add)) ^^
    (* Not using Tagged.load_field since it is not a proper pointer to the array start *)
    Heap.load_field Arr.header_size (* loads the element at the byte offset *)
  | GetLastArrayOffset, [e] ->
    assert (BitTagged.can_tag_const Type.Int (Int64.sub Arr.max_array_size 1L));
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^ (* array *)
    Arr.len env ^^
    compile_sub_const 1L ^^
    BigNum.from_signed_word_compact env

  | IdxBlobPrim, [e1; e2] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e1 ^^ (* offset to blob payload *)
    compile_exp_vanilla env ae e2 ^^ (* idx *)
    Blob.idx_bigint env

  | BreakPrim name, [e] ->
    let d = VarEnv.get_label_depth ae name in
    SR.Unreachable,
    compile_exp_vanilla env ae e ^^
    G.branch_to_ d
  | AssertPrim, [e1] ->
    SR.unit,
    compile_exp_as env ae SR.bool e1 ^^
    E.if0 G.nop (IC.fail_assert env at)
  | RetPrim, [e] ->
    SR.Unreachable,
    compile_exp_as env ae (StackRep.of_arity (E.get_return_arity env)) e ^^
    FakeMultiVal.store env (Lib.List.make (E.get_return_arity env) I64Type) ^^
    G.i Return

  (* Numeric conversions *)
  | NumConvWrapPrim (t1, t2), [e] -> begin
    let open Type in
    match t1, t2 with
    | (Nat|Int), (Nat8|Nat16|Nat32|Int8|Int16|Int32) ->
      SR.UnboxedWord64 t2, (* ! *)
      compile_exp_vanilla env ae e ^^
      Prim.prim_intToWordNShifted env (TaggedSmallWord.shift_of_type t2)

    | (Nat|Int), ((Nat64|Int64) as p) ->
      SR.UnboxedWord64 p,
      compile_exp_vanilla env ae e ^^
      BigNum.truncate_to_word64 env

    | Nat64, Int64 | Int64, Nat64 ->
      SR.UnboxedWord64 t2,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e
    | Nat32, Int32 | Int32, Nat32 ->
      SR.UnboxedWord64 t2,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e
    | Nat16, Int16 | Int16, Nat16
    | Nat8, Int8 | Int8, Nat8 ->
      SR.UnboxedWord64 t2,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e
    | Char, Nat32 ->
      SR.UnboxedWord64 Nat32,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e ^^
      TaggedSmallWord.lsb_adjust_codepoint env ^^
      TaggedSmallWord.msb_adjust Nat32

    | _ -> SR.Unreachable, todo_trap env "compile_prim_invocation" (Arrange_ir.prim p)
    end

  | NumConvTrapPrim (t1, t2), [e] -> begin
    let open Type in
    match t1, t2 with

    | Int, Int64 ->
      SR.UnboxedWord64 Int64,
      compile_exp_vanilla env ae e ^^
      Func.share_code1 Func.Never env "Int->Int64" ("n", I64Type) [I64Type] (fun env get_n ->
        get_n ^^
        BigNum.fits_signed_bits env 64 ^^
        E.else_trap_with env "losing precision" ^^
        get_n ^^
        BigNum.truncate_to_word64 env)

    | Int, (Int8|Int16|Int32 as pty) ->
      StackRep.of_type (Prim pty),
      compile_exp_vanilla env ae e ^^
      Func.share_code1 Func.Never env (prim_fun_name pty "Int->") ("n", I64Type) [I64Type] (fun env get_n ->
        get_n ^^
        BigNum.fits_signed_bits env (TaggedSmallWord.bits_of_type pty) ^^
        E.else_trap_with env "losing precision" ^^
        get_n ^^
        BigNum.truncate_to_word32 env ^^
        TaggedSmallWord.msb_adjust pty)

    | Nat, Nat64 ->
      SR.UnboxedWord64 Nat64,
      compile_exp_vanilla env ae e ^^
      Func.share_code1 Func.Never env "Nat->Nat64" ("n", I64Type) [I64Type] (fun env get_n ->
        get_n ^^
        BigNum.fits_unsigned_bits env 64 ^^
        E.else_trap_with env "losing precision" ^^
        get_n ^^
        BigNum.truncate_to_word64 env)

    | Nat, (Nat8|Nat16|Nat32 as pty) ->
      StackRep.of_type (Prim pty),
      compile_exp_vanilla env ae e ^^
      Func.share_code1 Func.Never env (prim_fun_name pty "Nat->") ("n", I64Type) [I64Type] (fun env get_n ->
        get_n ^^
        BigNum.fits_unsigned_bits env (TaggedSmallWord.bits_of_type pty) ^^
        E.else_trap_with env "losing precision" ^^
        get_n ^^
        BigNum.truncate_to_word32 env ^^
        TaggedSmallWord.msb_adjust pty)

    | (Nat8|Nat16|Nat32), Nat ->
      SR.Vanilla,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e ^^
      Prim.prim_shiftWordNtoUnsigned env (TaggedSmallWord.shift_of_type t1)

    | (Int8|Int16|Int32), Int ->
      SR.Vanilla,
      compile_exp_as env ae (SR.UnboxedWord64 t1) e ^^
      Prim.prim_shiftWordNtoSigned env (TaggedSmallWord.shift_of_type t1)

    | Nat64, Nat ->
      SR.Vanilla,
      compile_exp_as env ae (SR.UnboxedWord64 Nat64) e ^^
      BigNum.from_word64 env

    | Int64, Int ->
      SR.Vanilla,
      compile_exp_as env ae (SR.UnboxedWord64 Int64) e ^^
      BigNum.from_signed_word64 env

    | Nat32, Char ->
      SR.UnboxedWord64 Type.Char, (* ! *)
      compile_exp_as env ae (SR.UnboxedWord64 Nat32) e ^^
      TaggedSmallWord.lsb_adjust Nat32 ^^
      TaggedSmallWord.check_and_msb_adjust_codepoint env (* TBR *)

    | Float, Int ->
      SR.Vanilla,
      compile_exp_as env ae SR.UnboxedFloat64 e ^^
      E.call_import env "rts" "bigint_of_float64"

    | Int, Float ->
      SR.UnboxedFloat64,
      compile_exp_vanilla env ae e ^^
      let set_b, get_b = new_local env "b" in
      set_b ^^
      get_b ^^
      BitTagged.if_tagged_scalar env [F64Type]
        (get_b ^^
         BitTagged.untag __LINE__ env Type.Int ^^
         G.i (Convert (Wasm_exts.Values.F64 F64Op.ConvertSI64)))
        (get_b ^^
         E.call_import env "rts" "bigint_to_float64")

    | Float, Int64 ->
      SR.UnboxedWord64 Int64,
      compile_exp_as env ae SR.UnboxedFloat64 e ^^
      G.i (Convert (Wasm_exts.Values.I64 I64Op.TruncSF64))

    | Int64, Float ->
      SR.UnboxedFloat64,
      compile_exp_as env ae (SR.UnboxedWord64 Int64) e ^^
      G.i (Convert (Wasm_exts.Values.F64 F64Op.ConvertSI64))
    | (Nat8 as from_typ), (Nat16 as to_typ)
    | (Nat16 as from_typ), (Nat32 as to_typ)
    | (Nat32 as from_typ), (Nat64 as to_typ) ->
      SR.UnboxedWord64 to_typ,
      compile_exp_as env ae (SR.UnboxedWord64 from_typ) e ^^
      TaggedSmallWord.lsb_adjust from_typ ^^
      TaggedSmallWord.msb_adjust to_typ
    | (Nat16 as from_typ), (Nat8 as to_typ)
    | (Nat32 as from_typ), (Nat16 as to_typ)
    | (Nat64 as from_typ), (Nat32 as to_typ) ->
      SR.UnboxedWord64 to_typ,
      let num_bits = (TaggedSmallWord.bits_of_type to_typ) in
      let set_val, get_val = new_local env "convertee" in
      compile_exp_as env ae (SR.UnboxedWord64 from_typ) e ^^
      set_val ^^
      get_val ^^
      compile_shrU_const (Int64.of_int (64 - num_bits)) ^^
      E.then_trap_with env "losing precision" ^^
      get_val ^^
      compile_shl_const (Int64.of_int num_bits)
    | (Int8 as from_typ), (Int16 as to_typ)
    | (Int16 as from_typ), (Int32 as to_typ)
    | (Int32 as from_typ), (Int64 as to_typ) ->
      SR.UnboxedWord64 to_typ,
      compile_exp_as env ae (SR.UnboxedWord64 from_typ) e ^^
      TaggedSmallWord.lsb_adjust from_typ ^^
      TaggedSmallWord.msb_adjust to_typ
    | (Int16 as from_typ), (Int8 as to_typ)
    | (Int32 as from_typ), (Int16 as to_typ)
    | (Int64 as from_typ), (Int32 as to_typ) ->
      SR.UnboxedWord64 to_typ,
      let num_bits = (TaggedSmallWord.bits_of_type to_typ) in
      let set_val, get_val = new_local env "convertee" in
      compile_exp_as env ae (SR.UnboxedWord64 from_typ)  e ^^
      set_val ^^
      get_val ^^
      compile_shl_const (Int64.of_int num_bits) ^^
      compile_shrS_const (Int64.of_int num_bits) ^^
      get_val ^^
      compile_eq env Type.(Prim from_typ) ^^
      E.else_trap_with env "losing precision" ^^
      get_val ^^
      compile_shl_const (Int64.of_int num_bits)
    | _ -> SR.Unreachable, todo_trap env "compile_prim_invocation" (Arrange_ir.prim p)
    end

  | SerializePrim ts, [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Serialization.serialize env ts ^^
    Blob.of_ptr_size env

  | DeserializePrim ts, [e] ->
    StackRep.of_arity (List.length ts),
    compile_exp_vanilla env ae e ^^
    Bool.lit false ^^ (* can't recover *)
    Serialization.deserialize_from_blob false env ts

  | DeserializeOptPrim ts, [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Bool.lit true ^^ (* can (!) recover *)
    Serialization.deserialize_from_blob false env ts ^^
    begin match ts with
    | [] ->
      (* return some () *)
      Opt.inject env (Tuple.compile_unit env)
    | [t] ->
      (* save to local, propagate error as null or return some value *)
      let (set_val, get_val) = new_local env "val" in
      set_val ^^
      get_val ^^
      compile_eq_const (Serialization.coercion_error_value env) ^^
      E.if1 I64Type
        (Opt.null_lit env)
        (Opt.inject env get_val)
    | ts ->
      (* propagate any errors as null or return some tuples using shared code *)
      let n = List.length ts in
      let name = Printf.sprintf "to_opt_%i_tuple" n in
      let args = Lib.List.table n (fun i -> (Printf.sprintf "arg%i" i, I64Type)) in
      Func.share_code Func.Always env name args [I64Type] (fun env getters ->
        let locals =
          Lib.List.table n (fun i -> List.nth getters i) in
        let rec go ls =
          match ls with
          | get_val::ls' ->
            get_val ^^
            compile_eq_const (Serialization.coercion_error_value env) ^^
            E.if1 I64Type
              (Opt.null_lit env)
              (go ls')
          | [] ->
            Opt.inject env (Arr.lit env Tagged.T locals)
        in
        go locals)
    end

  | ICPerformGC, [] ->
    SR.unit,
    GC.collect_garbage env

  | ICStableSize t, [e] ->
    SR.UnboxedWord64 Type.Nat64,
    E.trap_with env "Deprecated with enhanced orthogonal persistence"

  (* Other prims, unary *)

  | OtherPrim "array_len", [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Arr.len env ^^
    BigNum.from_word64 env

  | OtherPrim "text_len", [e] ->
    SR.Vanilla, compile_exp_vanilla env ae e ^^ Text.len_nat env
  | OtherPrim "text_iter", [e] ->
    SR.Vanilla, compile_exp_vanilla env ae e ^^ Text.iter env
  | OtherPrim "text_iter_done", [e] ->
    SR.bool, compile_exp_vanilla env ae e ^^ Text.iter_done env
  | OtherPrim "text_iter_next", [e] ->
    SR.UnboxedWord64 Type.Char, compile_exp_vanilla env ae e ^^ Text.iter_next env
  | OtherPrim "text_compare", [e1; e2] ->
    SR.UnboxedWord64 Type.Int8,
    compile_exp_vanilla env ae e1 ^^
    compile_exp_vanilla env ae e2 ^^
    E.call_import env "rts" "text_compare" ^^
    TaggedSmallWord.msb_adjust Type.Int8
  | OtherPrim "blob_compare", [e1; e2] ->
    SR.UnboxedWord64 Type.Int8,
    compile_exp_vanilla env ae e1 ^^
    compile_exp_vanilla env ae e2 ^^
    Blob.compare env None ^^
    TaggedSmallWord.msb_adjust Type.Int8

  | OtherPrim "blob_size", [e] ->
    SR.Vanilla, compile_exp_vanilla env ae e ^^ Blob.len_nat env
  | OtherPrim "blob_vals_iter", [e] ->
    SR.Vanilla, compile_exp_vanilla env ae e ^^ Blob.iter env
  | OtherPrim "blob_iter_done", [e] ->
    SR.bool, compile_exp_vanilla env ae e ^^ Blob.iter_done env
  | OtherPrim "blob_iter_next", [e] ->
    SR.UnboxedWord64 Type.Nat8, (* ! *)
    compile_exp_vanilla env ae e ^^ Blob.iter_next env

  | OtherPrim "lsh_Nat", [e1; e2] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e2 ^^
    BigNum.compile_lsh env

  | OtherPrim "rsh_Nat", [e1; e2] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e2 ^^
    BigNum.compile_rsh env

  | OtherPrim ("explode_Nat16" | "explode_Int16" as pr), [e] ->
    SR.UnboxedTuple 2,
    let set, get = new_local env "e" in
    compile_exp_vanilla env ae e ^^
    TaggedSmallWord.untag env Type.(if pr = "explode_Nat16" then Nat16 else Int16) ^^
    set ^^ get ^^
    compile_bitand_const 0xFF00000000000000L ^^
    TaggedSmallWord.tag env Type.Nat8 ^^
    get ^^
    compile_shl_const 8L ^^
    TaggedSmallWord.tag env Type.Nat8

  | OtherPrim ("explode_Nat32" | "explode_Int32" as pr), [e] ->
    SR.UnboxedTuple 4,
    let set, get = new_local env "e" in
    let byte_at_bit b =
      get ^^
      compile_shrU_const b ^^
      compile_shl_const 56L ^^
      TaggedSmallWord.tag env Type.Nat8 in
    compile_exp_vanilla env ae e ^^
    TaggedSmallWord.untag env Type.(if pr = "explode_Nat32" then Nat32 else Int32) ^^
    set ^^ get ^^
    compile_bitand_const 0xFF00000000000000L ^^
    TaggedSmallWord.tag env Type.Nat8 ^^
    byte_at_bit 48L ^^
    byte_at_bit 40L ^^
    byte_at_bit 32L

  | OtherPrim ("explode_Nat64" | "explode_Int64" as pr), [e] ->
    SR.UnboxedTuple 8,
    let set, get = new_local env "e" in
    let byte_at_bit b =
      get ^^
      (if b = 0L then G.nop else compile_shrU_const b) ^^
      compile_shl_const 56L ^^
      TaggedSmallWord.tag env Type.Nat8 in
    compile_exp_as env ae (SR.UnboxedWord64 Type.(if pr = "explode_Nat64" then Nat64 else Int64)) e ^^
    set ^^ get ^^
    compile_bitand_const 0xFF00000000000000L ^^
    TaggedSmallWord.tag env Type.Nat8 ^^
    byte_at_bit 48L ^^
    byte_at_bit 40L ^^
    byte_at_bit 32L ^^
    byte_at_bit 24L ^^
    byte_at_bit 16L ^^
    byte_at_bit 8L ^^
    byte_at_bit 0L

  | OtherPrim "abs", [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    BigNum.compile_abs env

  | OtherPrim "fabs", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Abs))

  | OtherPrim "fsqrt", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Sqrt))

  | OtherPrim "fceil", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Ceil))

  | OtherPrim "ffloor", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Floor))

  | OtherPrim "ftrunc", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Trunc))

  | OtherPrim "fnearest", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    G.i (Unary (Wasm_exts.Values.F64 F64Op.Nearest))

  | OtherPrim "fmin", [e; f] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    compile_exp_as env ae SR.UnboxedFloat64 f ^^
    G.i (Binary (Wasm_exts.Values.F64 F64Op.Min))

  | OtherPrim "fmax", [e; f] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    compile_exp_as env ae SR.UnboxedFloat64 f ^^
    G.i (Binary (Wasm_exts.Values.F64 F64Op.Max))

  | OtherPrim "fcopysign", [e; f] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    compile_exp_as env ae SR.UnboxedFloat64 f ^^
    G.i (Binary (Wasm_exts.Values.F64 F64Op.CopySign))

  | OtherPrim "Float->Text", [e] ->
    SR.Vanilla,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    compile_unboxed_const (TaggedSmallWord.vanilla_lit Type.Nat8 6L) ^^
    compile_unboxed_const (TaggedSmallWord.vanilla_lit Type.Nat8 0L) ^^
    E.call_import env "rts" "float_fmt"

  | OtherPrim "fmtFloat->Text", [f; prec; mode] ->
    SR.Vanilla,
    compile_exp_as env ae SR.UnboxedFloat64 f ^^
    compile_exp_vanilla env ae prec ^^
    compile_exp_vanilla env ae mode ^^
    E.call_import env "rts" "float_fmt"

  | OtherPrim "fsin", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "sin"

  | OtherPrim "fcos", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "cos"

  | OtherPrim "ftan", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "tan"

  | OtherPrim "fasin", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "asin"

  | OtherPrim "facos", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "acos"

  | OtherPrim "fatan", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "atan"

  | OtherPrim "fatan2", [y; x] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 y ^^
    compile_exp_as env ae SR.UnboxedFloat64 x ^^
    E.call_import env "rts" "atan2"

  | OtherPrim "fexp", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "exp"

  | OtherPrim "flog", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.UnboxedFloat64 e ^^
    E.call_import env "rts" "log"

  (* Other prims, nullary *)

  | SystemTimePrim, [] ->
    SR.UnboxedWord64 Type.Nat64,
    IC.get_system_time env

  | OtherPrim "call_perform_status", [] ->
    SR.UnboxedWord64 Type.Nat32,
    IC.get_call_perform_status env ^^
    TaggedSmallWord.msb_adjust Type.Nat32

  | OtherPrim "call_perform_message", [] ->
    SR.Vanilla,
    IC.get_call_perform_message env

  | OtherPrim "rts_version", [] ->
    SR.Vanilla,
    E.call_import env "rts" "version"

  | OtherPrim "rts_heap_size", [] ->
    SR.Vanilla,
    Heap.get_heap_size env ^^ Prim.prim_word64toNat env

  | OtherPrim "rts_memory_size", [] ->
    SR.Vanilla,
    Heap.get_memory_size ^^ BigNum.from_word64 env

  | OtherPrim "rts_total_allocation", [] ->
    SR.Vanilla,
    Heap.get_total_allocation env ^^ BigNum.from_word64 env

  | OtherPrim "rts_reclaimed", [] ->
    SR.Vanilla,
    Heap.get_reclaimed env ^^ BigNum.from_word64 env

  | OtherPrim "rts_max_live_size", [] ->
    SR.Vanilla,
    Heap.get_max_live_size env ^^ BigNum.from_word64 env

  | OtherPrim "rts_max_stack_size", [] ->
    SR.Vanilla,
    Stack.get_max_stack_size env ^^ Prim.prim_word64toNat env

  | OtherPrim "rts_callback_table_count", [] ->
    SR.Vanilla,
    ContinuationTable.count env ^^ Prim.prim_word64toNat env

  | OtherPrim "rts_callback_table_size", [] ->
    SR.Vanilla,
    ContinuationTable.size env ^^ Prim.prim_word64toNat env

  | OtherPrim "rts_mutator_instructions", [] ->
    SR.Vanilla,
    GC.get_mutator_instructions env ^^ BigNum.from_word64 env

  | OtherPrim "rts_collector_instructions", [] ->
    SR.Vanilla,
    GC.get_collector_instructions env ^^ BigNum.from_word64 env

  | OtherPrim "rts_lifetime_instructions", [] ->
    SR.Vanilla,
    GC.get_lifetime_instructions env ^^ BigNum.from_word64 env

  | OtherPrim "rts_upgrade_instructions", [] ->
    SR.Vanilla,
    UpgradeStatistics.get_upgrade_instructions env ^^ BigNum.from_word64 env

  | OtherPrim "rts_stable_memory_size", [] ->
    SR.Vanilla,
    StableMem.stable64_size env ^^ BigNum.from_word64 env

  | OtherPrim "rts_logical_stable_memory_size", [] ->
    SR.Vanilla,
    StableMem.get_mem_size env ^^ BigNum.from_word64 env

  | OtherPrim "rts_in_upgrade", [] -> (* EOP specific *)
    assert (!Flags.enhanced_orthogonal_persistence);
    SR.Vanilla,
    Persistence.in_upgrade env

  | OtherPrim "alloc_weak_ref", [target] ->
    SR.Vanilla,
    WeakRef.try_inject env (compile_exp_vanilla env ae target) ^^
    E.call_import env "rts" "alloc_weak_ref"

  | OtherPrim "weak_get", [weak_ref] ->
    SR.Vanilla,
    compile_exp_vanilla env ae weak_ref ^^
    WeakRef.load_field env

  | OtherPrim "weak_ref_is_live", [weak_ref] ->
    SR.Vanilla,
    compile_exp_vanilla env ae weak_ref ^^
    E.call_import env "rts" "weak_ref_is_live" ^^
    Bool.from_rts_int32

  | OtherPrim "env_var_names", [] ->
    SR.Vanilla,
    IC.env_var_names env

  | OtherPrim "env_var", [name] ->
    SR.Vanilla,
    compile_exp_vanilla env ae name ^^
    IC.env_var env

  | OtherPrim "get_dedup_table", [] ->
    SR.Vanilla,
    E.call_import env "rts" "get_dedup_table"

  | OtherPrim "set_dedup_table", [dedup_table] ->
    SR.unit,
    compile_exp_vanilla env ae dedup_table ^^
    E.call_import env "rts" "set_dedup_table"

  (* Regions *)

  | OtherPrim "regionNew", [] ->
    SR.Vanilla,
    Region.new_ env

  | OtherPrim "regionId", [e0] ->
     SR.Vanilla,
     compile_exp_as env ae SR.Vanilla e0 ^^
     Region.id env ^^
     BigNum.from_word64 env

  | OtherPrim ("regionGrow"), [e0; e1] ->
    SR.UnboxedWord64 Type.Nat64,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.grow env

  | OtherPrim "regionSize", [e0] ->
    SR.UnboxedWord64 Type.Nat64,
    compile_exp_as env ae SR.Vanilla e0 ^^
    Region.size env

  | OtherPrim ("regionLoadBlob"), [e0; e1; e2] ->
    SR.Vanilla,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.Vanilla e2 ^^
    BigNum.to_word64_with env (Blob.lit env Tagged.T "Blob size out of bounds") ^^
    Region.load_blob env

  | OtherPrim ("regionStoreBlob"), [e0; e1; e2] ->
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.Vanilla e2 ^^
    Region.store_blob env

  | OtherPrim (("regionLoadNat8" | "regionLoadInt8" as p)), [e0; e1] ->
    let ty = Type.(if p = "regionLoadNat8" then Nat8 else Int8) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.load_word8 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat8 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  | OtherPrim (("regionStoreNat8" | "regionStoreInt8") as p), [e0; e1; e2] ->
    let ty = Type.(if p = "regionStoreNat8" then Nat8 else Int8) in
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    Region.store_word8 env

  | OtherPrim (("regionLoadNat16" | "regionLoadInt16") as p), [e0; e1] ->
    let ty = Type.(if p = "regionLoadNat16" then Nat16 else Int16) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.load_word16 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat16 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  | OtherPrim (("regionStoreNat16" | "regionStoreInt16") as p), [e0; e1; e2] ->
    let ty = Type.(if p = "regionStoreNat16" then Nat16 else Int16) in
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    Region.store_word16 env

  | OtherPrim (("regionLoadNat32" | "regionLoadInt32") as p), [e0; e1] ->
    let ty = Type.(if p = "regionLoadNat32" then Nat32 else Int32) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.load_word32 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat32 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  | OtherPrim (("regionStoreNat32" | "regionStoreInt32") as p), [e0; e1; e2] ->
    let ty = Type.(if p = "regionStoreNat32" then Nat32 else Int32) in
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    Region.store_word32 env

  | OtherPrim (("regionLoadNat64" | "regionLoadInt64") as p), [e0; e1] ->
    let ty = Type.(if p = "regionLoadNat64" then Nat64 else Int64) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.load_word64 env

  | OtherPrim (("regionStoreNat64" | "regionStoreInt64") as p), [e0; e1; e2] ->
    let ty = Type.(if p = "regionStoreNat64" then Nat64 else Int64) in
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    Region.store_word64 env

  | OtherPrim ("regionLoadFloat"), [e0; e1] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    Region.load_float64 env

  | OtherPrim ("regionStoreFloat"), [e0; e1; e2] ->
    SR.unit,
    compile_exp_as env ae SR.Vanilla e0 ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.UnboxedFloat64 e2 ^^
    Region.store_float64 env

  (* Other prims, unary *)

  | OtherPrim "global_timer_set", [e] ->
    SR.UnboxedWord64 Type.Nat64,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    IC.system_call env "global_timer_set"

  | OtherPrim "is_controller", [e] ->
    SR.Vanilla,
    let set_principal, get_principal = new_local env "principal" in
    compile_exp_vanilla env ae e ^^
    set_principal ^^ get_principal ^^
    Blob.payload_ptr_unskewed env ^^
    get_principal ^^
    Blob.len env ^^
    IC.is_controller env

  | OtherPrim "replicated_execution", [] ->
    SR.Vanilla,
    IC.replicated_execution env

  | OtherPrim "canister_version", [] ->
    SR.UnboxedWord64 Type.Nat64,
    IC.canister_version env

  | OtherPrim "crc32Hash", [e] ->
    SR.UnboxedWord64 Type.Nat32,
    compile_exp_vanilla env ae e ^^
    E.call_import env "rts" "compute_crc32" ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
    TaggedSmallWord.msb_adjust Type.Nat32

  | OtherPrim "idlHash", [e] ->
    SR.Vanilla,
    E.trap_with env "idlHash only implemented in interpreter"


  | OtherPrim "popcnt8", [e] ->
    SR.UnboxedWord64 Type.Nat8,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat8) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt)) ^^
    TaggedSmallWord.msb_adjust Type.Nat8
  | OtherPrim "popcnt16", [e] ->
    SR.UnboxedWord64 Type.Nat16,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat16) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt)) ^^
    TaggedSmallWord.msb_adjust Type.Nat16
  | OtherPrim "popcntInt8", [e] ->
    SR.UnboxedWord64 Type.Int8,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Int8) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt)) ^^
    TaggedSmallWord.msb_adjust Type.Int8
  | OtherPrim "popcntInt16", [e] ->
    SR.UnboxedWord64 Type.Int16,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Int16) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt)) ^^
    TaggedSmallWord.msb_adjust Type.Int16
  | OtherPrim "popcnt32", [e] ->
     SR.UnboxedWord64 Type.Nat32,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt)) ^^
     TaggedSmallWord.msb_adjust Type.Nat32
  | OtherPrim "popcntInt32", [e] ->
     SR.UnboxedWord64 Type.Int32,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int32) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt))^^
     TaggedSmallWord.msb_adjust Type.Int32
  | OtherPrim "popcnt64", [e] ->
     SR.UnboxedWord64 Type.Nat64,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt))
  | OtherPrim "popcntInt64", [e] ->
     SR.UnboxedWord64 Type.Int64,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int64) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Popcnt))
  | OtherPrim "clz8", [e] ->
     SR.UnboxedWord64 Type.Nat8,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat8) e ^^
     TaggedSmallWord.clz_kernel Type.Nat8
  | OtherPrim "clz16", [e] ->
     SR.UnboxedWord64 Type.Nat16,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat16) e ^^
     TaggedSmallWord.clz_kernel Type.Nat16
  | OtherPrim "clzInt8", [e] ->
     SR.UnboxedWord64 Type.Int8,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int8) e ^^
     TaggedSmallWord.clz_kernel Type.Int8
  | OtherPrim "clzInt16", [e] ->
     SR.UnboxedWord64 Type.Int16,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int16) e ^^
     TaggedSmallWord.clz_kernel Type.Int16
  | OtherPrim "clz32", [e] ->
     SR.UnboxedWord64 Type.Nat32,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e ^^
     TaggedSmallWord.clz_kernel Type.Nat32
  | OtherPrim "clzInt32", [e] ->
     SR.UnboxedWord64 Type.Int32,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int32) e ^^
     TaggedSmallWord.clz_kernel Type.Int32
  | OtherPrim "clz64", [e] ->
     SR.UnboxedWord64 Type.Nat64,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz))
  | OtherPrim "clzInt64", [e] ->
     SR.UnboxedWord64 Type.Int64,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int64) e ^^
     G.i (Unary (Wasm_exts.Values.I64 I64Op.Clz))
  | OtherPrim "ctz8", [e] ->
     SR.UnboxedWord64 Type.Nat8,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat8) e ^^
     TaggedSmallWord.ctz_kernel Type.Nat8
  | OtherPrim "ctz16", [e] ->
     SR.UnboxedWord64 Type.Nat16,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Nat16) e ^^
     TaggedSmallWord.ctz_kernel Type.Nat16
  | OtherPrim "ctzInt8", [e] ->
     SR.UnboxedWord64 Type.Int8,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int8) e ^^
     TaggedSmallWord.ctz_kernel Type.Int8
  | OtherPrim "ctzInt16", [e] ->
     SR.UnboxedWord64 Type.Int16,
     compile_exp_as env ae (SR.UnboxedWord64 Type.Int16) e ^^
     TaggedSmallWord.ctz_kernel Type.Int16
  | OtherPrim "ctz32", [e] ->
    SR.UnboxedWord64 Type.Nat32,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e ^^
    TaggedSmallWord.ctz_kernel Type.Nat32
  | OtherPrim "ctzInt32", [e] ->
    SR.UnboxedWord64 Type.Int32,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Int32) e ^^
    TaggedSmallWord.ctz_kernel Type.Int32
  | OtherPrim "ctz64", [e] ->
    SR.UnboxedWord64 Type.Nat64,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Ctz))
  | OtherPrim "ctzInt64", [e] ->
    SR.UnboxedWord64 Type.Int64,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Int64) e ^^
    G.i (Unary (Wasm_exts.Values.I64 I64Op.Ctz))

  | OtherPrim "conv_Char_Text", [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Text.prim_showChar env

  | OtherPrim "char_to_upper", [e] ->
    compile_char_to_char_rts env ae e "char_to_upper"

  | OtherPrim "char_to_lower", [e] ->
    compile_char_to_char_rts env ae e "char_to_lower"

  | OtherPrim "char_is_whitespace", [e] ->
    compile_char_to_bool_rts env ae e "char_is_whitespace"

  | OtherPrim "char_is_lowercase", [e] ->
    compile_char_to_bool_rts env ae e "char_is_lowercase"

  | OtherPrim "char_is_uppercase", [e] ->
    compile_char_to_bool_rts env ae e "char_is_uppercase"

  | OtherPrim "char_is_alphabetic", [e] ->
    compile_char_to_bool_rts env ae e "char_is_alphabetic"

  | OtherPrim "print", [e] ->
    SR.unit,
    compile_exp_vanilla env ae e ^^
    IC.print_text env

  | OtherPrim "text_lowercase", [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Text.lowercase env

  | OtherPrim "text_uppercase", [e] ->
    SR.Vanilla,
    compile_exp_vanilla env ae e ^^
    Text.uppercase env

  | OtherPrim "performanceCounter", [e] ->
    (SR.UnboxedWord64 Type.Nat64),
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e ^^
    IC.performance_counter env

  | OtherPrim "trap", [e] ->
    SR.Unreachable,
    compile_exp_vanilla env ae e ^^
    IC.trap_text env

  | OtherPrim "principalOfBlob", e ->
    const_sr SR.Vanilla (Blob.copy env Tagged.B Tagged.P)
  | OtherPrim "blobOfPrincipal", e ->
    const_sr SR.Vanilla (Blob.copy env Tagged.P Tagged.B)
  | OtherPrim "principalOfActor", e ->
    const_sr SR.Vanilla (Blob.copy env Tagged.A Tagged.P)

  | OtherPrim "blobToArray", e ->
    const_sr SR.Vanilla (Arr.ofBlob env Tagged.I)
  | OtherPrim "blobToArrayMut", e ->
    const_sr SR.Vanilla (Arr.ofBlob env Tagged.M)

  | OtherPrim ("arrayToBlob" | "arrayMutToBlob"), e ->
    const_sr SR.Vanilla (Arr.toBlob env)

  | OtherPrim (("stableMemoryLoadNat32" | "stableMemoryLoadInt32") as p), [e] ->
    let ty = Type.(if p = "stableMemoryLoadNat32" then Nat32 else Int32) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.load_word32 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat32 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  | OtherPrim (("stableMemoryStoreNat32" | "stableMemoryStoreInt32") as p), [e1; e2] ->
    let ty = Type.(if p = "stableMemoryStoreNat32" then Nat32 else Int32) in
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    StableMemoryInterface.store_word32 env

  | OtherPrim (("stableMemoryLoadNat8" | "stableMemoryLoadInt8") as p), [e] ->
    let ty = Type.(if p = "stableMemoryLoadNat8" then Nat8 else Int8) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.load_word8 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat8 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  (* Other prims, binary *)

  | OtherPrim (("stableMemoryStoreNat8" | "stableMemoryStoreInt8") as p), [e1; e2] ->
    let ty = Type.(if p = "stableMemoryStoreNat8" then Nat8 else Int8) in
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    StableMemoryInterface.store_word8 env

  | OtherPrim (("stableMemoryLoadNat16" | "stableMemoryLoadInt16") as p), [e] ->
    let ty = Type.(if p = "stableMemoryLoadNat16" then Nat16 else Int16) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.load_word16 env ^^
    G.i (Convert (Wasm_exts.Values.I64 I64Op.(if ty = Type.Nat16 then ExtendUI32 else ExtendSI32))) ^^
    TaggedSmallWord.msb_adjust ty

  | OtherPrim (("stableMemoryStoreNat16" | "stableMemoryStoreInt16") as p), [e1; e2] ->
    let ty = Type.(if p = "stableMemoryStoreNat16" then Nat16 else Int16) in
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    TaggedSmallWord.lsb_adjust ty ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    StableMemoryInterface.store_word16 env

  | OtherPrim (("stableMemoryLoadNat64" | "stableMemoryLoadInt64") as p), [e] ->
    let ty = Type.(if p = "stableMemoryLoadNat64" then Nat64 else Int64) in
    SR.UnboxedWord64 ty,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.load_word64 env

  | OtherPrim (("stableMemoryStoreNat64" | "stableMemoryStoreInt64") as p), [e1; e2] ->
    let ty = Type.(if p = "stableMemoryStoreNat64" then Nat64 else Int64) in
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae (SR.UnboxedWord64 ty) e2 ^^
    StableMemoryInterface.store_word64 env

  | OtherPrim "stableMemoryLoadFloat", [e] ->
    SR.UnboxedFloat64,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.load_float64 env

  | OtherPrim "stableMemoryStoreFloat", [e1; e2] ->
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.UnboxedFloat64 e2 ^^
    StableMemoryInterface.store_float64 env

  | OtherPrim "stableMemoryLoadBlob", [e1; e2] ->
    SR.Vanilla,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.Vanilla e2 ^^
    BigNum.to_word64_with env (Blob.lit env Tagged.T "Blob size out of bounds") ^^
    StableMemoryInterface.load_blob env

  | OtherPrim "stableMemoryStoreBlob", [e1; e2] ->
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e1 ^^
    compile_exp_as env ae SR.Vanilla e2 ^^
    StableMemoryInterface.store_blob env

  | OtherPrim "stableMemorySize", [] ->
    SR.UnboxedWord64 Type.Nat64,
    StableMemoryInterface.size env

  | OtherPrim "stableMemoryGrow", [e] ->
    SR.UnboxedWord64 Type.Nat64,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) e ^^
    StableMemoryInterface.grow env

  | OtherPrim "stableVarQuery", [] ->
    SR.Vanilla,
    IC.get_self_reference env ^^
    IC.actor_public_field env Type.(motoko_stable_var_info_fld.lab)

  | OtherPrim "canister_subnet", [] ->
    SR.Vanilla, IC.get_subnet_reference env

  | OtherPrim "root_key", [] ->
    SR.Vanilla, IC.get_root_key env

  | OtherPrim "canister_self", [] ->
    SR.Vanilla, IC.get_self_reference env

  (* Other prims, binary *)
  | OtherPrim "Array.init", [_;_] ->
    const_sr SR.Vanilla (Arr.init env)
  | OtherPrim "Array.tabulate", [_;_] ->
    const_sr SR.Vanilla (Arr.tabulate env Tagged.I)
  | OtherPrim "Array.tabulateVar", [_;_] ->
    const_sr SR.Vanilla (Arr.tabulate env Tagged.M)
  | OtherPrim "btst8", [_;_] ->
    (* TODO: btstN returns Bool, not a small value *)
    const_sr (SR.UnboxedWord64 Type.Nat8) (TaggedSmallWord.btst_kernel env Type.Nat8)
  | OtherPrim "btst16", [_;_] ->
    const_sr (SR.UnboxedWord64 Type.Nat16) (TaggedSmallWord.btst_kernel env Type.Nat16)
  | OtherPrim "btstInt8", [_;_] ->
    const_sr (SR.UnboxedWord64 Type.Int8) (TaggedSmallWord.btst_kernel env Type.Int8)
  | OtherPrim "btstInt16", [_;_] ->
    const_sr (SR.UnboxedWord64 Type.Int16) (TaggedSmallWord.btst_kernel env Type.Int16)
  | OtherPrim "btst32", [_;_] ->
     const_sr (SR.UnboxedWord64 Type.Nat32) (TaggedSmallWord.btst_kernel env Type.Nat32)
  | OtherPrim "btstInt32", [_;_] ->
     const_sr (SR.UnboxedWord64 Type.Int32) (TaggedSmallWord.btst_kernel env Type.Int32) (* ! *)
  | OtherPrim "btst64", [_;_] ->
    const_sr (SR.UnboxedWord64 Type.Nat64) (Word64.btst_kernel env)
  | OtherPrim "btstInt64", [_;_] ->
    const_sr (SR.UnboxedWord64 Type.Int64) (Word64.btst_kernel env)

  | OtherPrim "setCandidLimits", [e1; e2; e3] ->
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e1 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    Serialization.Registers.set_value_numerator env ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e2 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    Serialization.Registers.set_value_denominator env ^^
    Serialization.Registers.get_value_denominator env ^^
    E.else_trap_with env "Candid limit denominator cannot be zero" ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e3 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    Serialization.Registers.set_value_bias env

  | OtherPrim "getCandidLimits", [] ->
    SR.UnboxedTuple 3,
    Serialization.Registers.get_value_numerator env ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    TaggedSmallWord.tag env Type.Nat32 ^^
    Serialization.Registers.get_value_denominator env ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    TaggedSmallWord.tag env Type.Nat32 ^^
    Serialization.Registers.get_value_bias env ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    TaggedSmallWord.tag env Type.Nat32

  | OtherPrim "setCandidTypeLimits", [e1; e2] ->
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e1 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    Serialization.Registers.set_type_scaler env ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e2 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    Serialization.Registers.set_type_bias env

  | OtherPrim "getCandidTypeLimits", [] ->
    SR.UnboxedTuple 2,
    Serialization.Registers.get_type_scaler env ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    TaggedSmallWord.tag env Type.Nat32 ^^
    Serialization.Registers.get_type_bias env ^^
    TaggedSmallWord.msb_adjust Type.Nat32 ^^
    TaggedSmallWord.tag env Type.Nat32

  (* Coercions for abstract types *)
  | CastPrim (_,_), [e] ->
    compile_exp env ae e

  | DecodeUtf8, [_] ->
    const_sr SR.Vanilla (Text.of_blob env)
  | EncodeUtf8, [_] ->
    const_sr SR.Vanilla (Text.to_blob env)

  (* textual to bytes *)
  | BlobOfIcUrl, [_] ->
    const_sr SR.Vanilla (E.call_import env "rts" "blob_of_principal")
  (* The other direction *)
  | IcUrlOfBlob, [_] ->
    const_sr SR.Vanilla (E.call_import env "rts" "principal_of_blob")

  (* Actor ids are blobs in the RTS *)
  | ActorOfIdBlob _, [e] ->
    SR.Vanilla,
    let (set_blob, get_blob) = new_local env "blob" in
    compile_exp_vanilla env ae e ^^
    set_blob ^^
    get_blob ^^
    Blob.len env ^^
    compile_unboxed_const 29L ^^
    compile_comparison I64Op.LeU ^^
    E.else_trap_with env "blob too long for actor principal" ^^
    get_blob ^^
    Blob.copy env Tagged.B Tagged.A

  | SelfRef _, [] ->
    SR.Vanilla, IC.get_self_reference env

  | ICArgDataPrim, [] ->
    SR.Vanilla, IC.arg_data env

  | ICReplyPrim ts, [e] ->
    SR.unit, begin match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      compile_exp_vanilla env ae e ^^
      (* For raw HTTP handlers, skip Candid serialization *)
      (if !(E.(env.compiling_raw_handler)) then
        (* Raw mode: convert [Nat8] to Blob if needed, then send directly *)
        let needs_conversion = match ts with
         | [t] ->
           let unwrapped = Type.as_immut (Type.normalize t) in
           (match unwrapped with
            | Type.Array elem_type ->
              (match Type.normalize elem_type with
               | Type.Prim Type.Nat8 -> true
               | _ -> false)
            | _ -> false)
         | _ -> false
        in
        (if needs_conversion then Arr.toBlob env else G.nop) ^^
        Blob.as_ptr_len env
       else
        (* Normal mode: serialize to Candid *)
        Serialization.serialize env ts) ^^
      IC.reply_with_data env
    | _ ->
      E.trap_with env "cannot reply when running locally"
    end

  | ICRejectPrim, [e] ->
    SR.unit, IC.reject env (compile_exp_vanilla env ae e)

  | ICCallerPrim, [] ->
    SR.Vanilla, IC.caller env

  | ICCallPrim, [f;e;k;r;c] ->
    SR.unit, begin
    (* TBR: Can we do better than using the notes? *)
    let _, _, _, ts1, _ = Type.as_func f.note.Note.typ in
    let _, _, _, ts2, _ = Type.as_func k.note.Note.typ in
    let (set_meth_pair, get_meth_pair) = new_local env "meth_pair" in
    let (set_arg, get_arg) = new_local env "arg" in
    let (set_k, get_k) = new_local env "k" in
    let (set_r, get_r) = new_local env "r" in
    let (set_c, get_c) = new_local env "c" in
    let add_cycles = Internals.add_cycles env ae in
    compile_exp_vanilla env ae f ^^ set_meth_pair ^^
    compile_exp_vanilla env ae e ^^ set_arg ^^
    compile_exp_vanilla env ae k ^^ set_k ^^
    compile_exp_vanilla env ae r ^^ set_r ^^
    compile_exp_vanilla env ae c ^^ set_c ^^
    FuncDec.ic_call env ts1 ts2 get_meth_pair get_arg get_k get_r get_c add_cycles
    end

  | ICCallRawPrim, [p;m;a;k;r;c] ->
    SR.unit, begin
    let set_meth_pair, get_meth_pair = new_local env "meth_pair" in
    let set_arg, get_arg = new_local env "arg" in
    let set_k, get_k = new_local env "k" in
    let set_r, get_r = new_local env "r" in
    let set_c, get_c = new_local env "c" in
    let add_cycles = Internals.add_cycles env ae in
    compile_exp_vanilla env ae p ^^
    compile_exp_vanilla env ae m ^^ Text.to_blob env ^^
    Tagged.load_forwarding_pointer env ^^
    Tuple.from_stack env 2 ^^ set_meth_pair ^^
    compile_exp_vanilla env ae a ^^ set_arg ^^
    compile_exp_vanilla env ae k ^^ set_k ^^
    compile_exp_vanilla env ae r ^^ set_r ^^
    compile_exp_vanilla env ae c ^^ set_c ^^
    FuncDec.ic_call_raw env get_meth_pair get_arg get_k get_r get_c add_cycles
    end

  | ICMethodNamePrim, [] ->
    SR.Vanilla, IC.method_name env

  | ICReplyDeadlinePrim, [] ->
    SR.UnboxedWord64 Type.Nat64, IC.deadline env

  | ICStableRead ty, [] ->
    SR.Vanilla,
    Persistence.load env ty
  | ICStableWrite ty, [] ->
    SR.unit,
    Persistence.save env ty

  (* Cycles *)
  | SystemCyclesBalancePrim, [] ->
    SR.Vanilla, Cycles.balance env
  | SystemCyclesAddPrim, [e1] ->
    SR.unit, compile_exp_vanilla env ae e1 ^^ Cycles.add env
  | SystemCyclesAcceptPrim, [e1] ->
    SR.Vanilla, compile_exp_vanilla env ae e1 ^^ Cycles.accept env
  | SystemCyclesAvailablePrim, [] ->
    SR.Vanilla, Cycles.available env
  | SystemCyclesRefundedPrim, [] ->
    SR.Vanilla, Cycles.refunded env
  | SystemCyclesBurnPrim, [e1] ->
    SR.Vanilla, compile_exp_vanilla env ae e1 ^^ Cycles.burn env

  (* Cost *)
  | OtherPrim "costCall", [method_name_size; payload_size] ->
    SR.Vanilla,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) method_name_size ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) payload_size ^^
    Cost.call env
  | OtherPrim "costCreateCanister", [] ->
    SR.Vanilla, Cost.create_canister env
  | OtherPrim "costHttpRequest", [request_size; max_res_bytes] ->
    SR.Vanilla,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) request_size ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat64) max_res_bytes ^^
    Cost.http_request env
  | OtherPrim "costSignWithEcdsa", [key_name; curve] ->
    SR.UnboxedTuple 2,
    compile_exp_vanilla env ae key_name ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) curve ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    Cost.sign_with_ecdsa env
  | OtherPrim "costSignWithSchnorr", [key_name; algorithm] ->
    SR.UnboxedTuple 2,
    compile_exp_vanilla env ae key_name ^^
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) algorithm ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    Cost.sign_with_schnorr env

  | SystemTimeoutSetPrim, [e1] ->
    SR.unit,
    compile_exp_as env ae (SR.UnboxedWord64 Type.Nat32) e1 ^^
    TaggedSmallWord.lsb_adjust Type.Nat32 ^^
    G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
    IC.system_call env "call_with_best_effort_response"

  | SetCertifiedData, [e1] ->
    SR.unit, compile_exp_vanilla env ae e1 ^^ IC.set_certified_data env
  | GetCertificate, [] ->
    SR.Vanilla,
    IC.get_certificate env

  (* Unknown prim *)
  | _ -> SR.Unreachable, todo_trap env "compile_prim_invocation" (Arrange_ir.prim p)
  end

(* Compile, infer and return stack representation *)
and compile_exp (env : E.t) ae exp =
  compile_exp_with_hint env ae None exp

(* Compile to given stack representation *)
and compile_exp_as env ae sr_out e =
  let sr_in, code = compile_exp_with_hint env ae (Some sr_out) e in
  code ^^ StackRep.adjust env sr_in sr_out

and single_case e (cs : Ir.case list) =
  match cs, e.note.Note.typ with
  | [{it={pat={it=TagP (l, _);_}; _}; _}], Type.(Variant [{lab; _}]) -> l = lab
  | _ -> false

and known_tag_pat p = TagP ("", p)

and simplify_cases e (cs : Ir.case list) =
  match cs, e.note.Note.typ with
  (* for a 2-cased variant type, the second comparison can be omitted when the first pattern
     (with irrefutable subpattern) didn't match, and the pattern types line up *)
  | [{it={pat={it=TagP (l1, ip); _}; _}; _} as c1; {it={pat={it=TagP (l2, pat'); _} as pat2; exp}; _} as c2], Type.(Variant [{lab=el1; _}; {lab=el2; _}])
       when Ir_utils.is_irrefutable ip
            && (l1 = el1 || l1 = el2)
            && (l2 = el1 || l2 = el2) ->
     [c1; {c2 with it = {exp; pat = {pat2 with it = known_tag_pat pat'}}}]
  | _ -> cs

(* Compile, infer and return stack representation, taking the hint into account *)
and compile_exp_with_hint (env : E.t) ae sr_hint exp =
  (fun (sr,code) -> (sr, G.with_region exp.at code)) @@
  if exp.note.Note.const
  then let (c, fill) = compile_const_exp env ae exp None in fill env ae; (SR.Const c, G.nop)
  else match exp.it with
  | PrimE (p, es) when List.exists (fun e -> Type.is_non e.note.Note.typ) es ->
    (* Handle dead code separately, so that we can rely on useful type
       annotations below *)
    SR.Unreachable,
    G.concat_map (compile_exp_ignore env ae) es ^^
    G.i Unreachable

  | PrimE (p, es) ->
    compile_prim_invocation (env : E.t) ae p es exp.at
  | VarE (_, var) ->
    Var.get_val env ae var
  | AssignE (e1,e2) ->
    SR.unit,
    let (prepare_code, sr, store_code) = compile_lexp env ae e1 in
    prepare_code ^^
    compile_exp_as env ae sr e2 ^^
    store_code
  | LitE l ->
    compile_lit l
  | IfE (scrut, e1, e2) ->
    let code_scrut = compile_exp_as_test env ae scrut in
    let sr1, code1 = compile_exp_with_hint env ae sr_hint e1 in
    let sr2, code2 = compile_exp_with_hint env ae sr_hint e2 in
    (* Use the expected stackrep, if given, else infer from the branches *)
    let sr = match sr_hint with
      | Some sr -> sr
      | None -> StackRep.join sr1 sr2
    in
    sr,
    code_scrut ^^
    FakeMultiVal.if_ env
      (StackRep.to_block_type env sr)
      (code1 ^^ StackRep.adjust env sr1 sr)
      (code2 ^^ StackRep.adjust env sr2 sr)
  | BlockE (decs, exp) ->
    let captured = Freevars.captured_vars (Freevars.exp exp) in
    let ae', codeW1 = compile_decs env ae decs captured in
    let (sr, code2) = compile_exp_with_hint env ae' sr_hint exp in
    (sr, codeW1 code2)
  | LabelE (name, _ty, e) ->
    (* The value here can come from many places -- the expression,
       or any of the nested returns. Hard to tell which is the best
       stack representation here.
       So let’s go with Vanilla. *)
    SR.Vanilla,
    E.block_ env (StackRep.to_block_type env SR.Vanilla) (
      G.with_current_depth (fun depth ->
        let ae1 = VarEnv.add_label ae name depth in
        compile_exp_vanilla env ae1 e
      )
    )
  | LoopE e ->
    SR.Unreachable,
    let ae' = VarEnv.{ ae with lvl = NotTopLvl } in
    G.loop0 (compile_exp_unit env ae' e ^^ G.i (Br (nr 0l))
    )
    ^^
   G.i Unreachable

  | SwitchE (e, cs) when single_case e cs ->
    let code1 = compile_exp_vanilla env ae e in
    let [@warning "-8"] [{it={pat={it=TagP (_, pat');_} as pat; exp}; _}] = cs in
    let ae1, pat_code = compile_pat_local env ae {pat with it = known_tag_pat pat'} in
    let sr, rhs_code = compile_exp_with_hint env ae1 sr_hint exp in

    (* Use the expected stackrep, if given, else infer from the branches *)
    let final_sr = match sr_hint with
      | Some sr -> sr
      | None -> sr
    in

    final_sr,
    (* Run rest in block to exit from *)
    FakeMultiVal.block_ env (StackRep.to_block_type env final_sr) (fun branch_code ->
       orsPatternFailure env (List.map (fun (sr, c) ->
          c ^^^ CannotFail (StackRep.adjust env sr final_sr ^^ branch_code)
       ) [sr, CannotFail code1 ^^^ pat_code ^^^ CannotFail rhs_code]) ^^
       G.i Unreachable (* We should always exit using the branch_code *)
    )

  | SwitchE (e, cs) ->
    let code1 = compile_exp_vanilla env ae e in
    let (set_i, get_i) = new_local env "switch_in" in

    (* compile subexpressions and collect the provided stack reps *)
    let codes = List.map (fun {it={pat; exp=e}; _} ->
      let (ae1, pat_code) = compile_pat_local env ae pat in
      let (sr, rhs_code) = compile_exp_with_hint env ae1 sr_hint e in
      (sr, CannotFail get_i ^^^ pat_code ^^^ CannotFail rhs_code)
      ) (simplify_cases e cs) in

    (* Use the expected stackrep, if given, else infer from the branches *)
    let final_sr = match sr_hint with
      | Some sr -> sr
      | None -> StackRep.joins (List.map fst codes)
    in

    final_sr,
    (* Run scrut *)
    code1 ^^ set_i ^^
    (* Run rest in block to exit from *)
    FakeMultiVal.block_ env (StackRep.to_block_type env final_sr) (fun branch_code ->
       orsPatternFailure env (List.map (fun (sr, c) ->
          c ^^^ CannotFail (StackRep.adjust env sr final_sr ^^ branch_code)
       ) codes) ^^
       G.i Unreachable (* We should always exit using the branch_code *)
    )
  (* Async-wait lowering support features *)
  | DeclareE (name, typ, e) ->
    let ae1, i = VarEnv.add_local_with_heap_ind env ae name typ in
    let sr, code = compile_exp env ae1 e in
    sr,
    MutBox.alloc env ^^ G.i (LocalSet (nr i)) ^^
    code
  | DefineE (name, _, e) ->
    SR.unit,
    let pre_code, sr, code = Var.set_val env ae name in
    pre_code ^^
    compile_exp_as env ae sr e ^^
    code
  | FuncE (x, sort, control, typ_binds, args, res_tys, e) ->
    let captured = Freevars.captured exp in
    let return_tys = match control with
      | Type.Returns -> res_tys
      | Type.Replies -> []
      | Type.Promises -> assert false in
    let return_arity = List.length return_tys in
    let mk_body env1 ae1 = compile_exp_as env1 ae1 (StackRep.of_arity return_arity) e in
    FuncDec.lit env ae x sort control captured args mk_body return_tys exp.at
  | SelfCallE (ts, exp_f, exp_k, exp_r, exp_c) ->
    SR.unit,
    let (set_future, get_future) = new_local env "future" in
    let (set_k, get_k) = new_local env "k" in
    let (set_r, get_r) = new_local env "r" in
    let (set_c, get_c) = new_local env "c" in
    let mk_body env1 ae1 = compile_exp_as env1 ae1 SR.unit exp_f in
    let captured = Freevars.captured exp_f in
    let add_cycles = Internals.add_cycles env ae in
    FuncDec.async_body env ae ts captured mk_body exp.at ^^
    Tagged.load_forwarding_pointer env ^^
    set_future ^^

    compile_exp_vanilla env ae exp_k ^^ set_k ^^
    compile_exp_vanilla env ae exp_r ^^ set_r ^^
    compile_exp_vanilla env ae exp_c ^^ set_c ^^

    FuncDec.ic_self_call env ts
      IC.(get_self_reference env ^^
          actor_public_field env async_method_name)
      get_future
      get_k
      get_r
      get_c
      add_cycles
  | ActorE (ds, fs, _, _) ->
    fatal "Local actors not supported by backend"
  | NewObjE (Type.(Object | Module | Memory) as _sort, fs, _) ->
    (*
    We can enable this warning once we treat everything as static that
    mo_frontend/static.ml accepts, including _all_ literals.
    if sort = Type.Module then Printf.eprintf "%s" "Warning: Non-static module\n";
    *)
    SR.Vanilla,
    let fs' = fs |> List.map
      (fun (f : Ir.field) -> (f.it.name, fun () ->
        if Type.is_mut f.note
        then Var.get_aliased_box env ae f.it.var
        else Var.get_val_vanilla env ae f.it.var)) in
    Object.lit_raw env fs'
  | _ -> SR.unit, todo_trap env "compile_exp" (Arrange_ir.exp exp)

and compile_exp_ignore env ae e =
  let sr, code = compile_exp env ae e in
  code ^^ StackRep.drop env sr

and compile_exp_as_opt env ae sr_out_o e =
  let sr_in, code = compile_exp_with_hint env ae sr_out_o e in
  G.with_region e.at (
    code ^^
    match sr_out_o with
    | None -> StackRep.drop env sr_in
    | Some sr_out -> StackRep.adjust env sr_in sr_out
  )

and compile_exp_vanilla (env : E.t) ae exp =
  compile_exp_as env ae SR.Vanilla exp

and compile_exp_unit (env : E.t) ae exp =
  compile_exp_as env ae SR.unit exp

(* compiles to something that works with IfE or Eqz
   (SR.UnboxedWord64 or SR.Vanilla are _both_ ok)
*)
and compile_exp_as_test env ae e =
  let sr, code = compile_exp env ae e in
  code ^^
  (if sr != SR.bool then StackRep.adjust env sr SR.Vanilla else G.nop)

(* Compile a prim of type Char -> Char to a RTS call. *)
and compile_char_to_char_rts env ae exp rts_fn =
  SR.UnboxedWord64 Type.Char,
  compile_exp_as env ae (SR.UnboxedWord64 Type.Char) exp ^^
  TaggedSmallWord.lsb_adjust_codepoint env ^^
  G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
  E.call_import env "rts" rts_fn ^^
  G.i (Convert (Wasm_exts.Values.I64 I64Op.ExtendUI32)) ^^
  TaggedSmallWord.msb_adjust_codepoint


(* Compile a prim of type Char -> Bool to a RTS call. The RTS function should
   have type int32_t -> int32_t where the return value is 0 for 'false' and 1
   for 'true'. *)
and compile_char_to_bool_rts (env : E.t) (ae : VarEnv.t) exp rts_fn =
  SR.bool,
  compile_exp_as env ae (SR.UnboxedWord64 Type.Char) exp ^^
  TaggedSmallWord.lsb_adjust_codepoint env ^^
  G.i (Convert (Wasm_exts.Values.I32 I32Op.WrapI64)) ^^
  (* The RTS function returns Motoko True/False values (which are represented as
     1 and 0, respectively) so we don't need any marshalling *)
  E.call_import env "rts" rts_fn ^^
  Bool.from_rts_int32

(*
The compilation of declarations (and patterns!) needs to handle mutual recursion.
This requires conceptually three passes:
 1. First we need to collect all names bound in a block,
    and find locations for then (which extends the environment).
    The environment is extended monotonically: The type-checker ensures that
    a Block does not bind the same name twice.
    We would not need to pass in the environment, just out ... but because
    it is bundled in the E.t type, threading it through is also easy.

 2. We need to allocate memory for them, and store the pointer in the
    WebAssembly local, so that they can be captured by closures.

 3. We go through the declarations, generate the actual code and fill the
    allocated memory.
    This includes creating the actual closure references.

We could do this in separate functions, but I chose to do it in one
 * it means all code related to one constructor is in one place and
 * when generating the actual code, we still “know” the id of the local that
   has the memory location, and don’t have to look it up in the environment.

The first phase works with the `pre_env` passed to `compile_dec`,
while the third phase is a function that expects the final environment. This
enabled mutual recursion.
*)


and compile_lit_pat env l =
  match l with
  | NullLit ->
    Opt.is_null env
  | BoolLit true ->
    G.nop
  | BoolLit false ->
    compile_test I64Op.Eqz
  | (NatLit _ | IntLit _) ->
    compile_lit_as env SR.Vanilla l ^^
    BigNum.compile_eq env
  | Nat8Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Nat8)
  | Nat16Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Nat16)
  | Nat32Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.nat32
  | Nat64Lit _ ->
    BoxedWord64.unbox env Type.Nat64 ^^
    compile_lit_as env (SR.UnboxedWord64 Type.Nat64) l ^^
    compile_eq env Type.(Prim Nat64)
  | Int8Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Int8)
  | Int16Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Int16)
  | Int32Lit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Int32)
  | Int64Lit _ ->
    BoxedWord64.unbox env Type.Int64 ^^
    compile_lit_as env (SR.UnboxedWord64 Type.Int64) l ^^
    compile_eq env Type.(Prim Int64)
  | CharLit _ ->
    compile_lit_as env SR.Vanilla l ^^
    compile_eq env Type.(Prim Char)
  | TextLit t ->
    compile_lit_as env SR.Vanilla l ^^
    Text.compare env Operator.EqOp
  | BlobLit t ->
    compile_lit_as env SR.Vanilla l ^^
    Blob.compare env (Some Operator.EqOp)
  | FloatLit _ ->
    todo_trap env "compile_lit_pat" (Arrange_ir.lit l)

and fill_pat env ae pat : patternCode =
  PatCode.with_region pat.at @@
  match pat.it with
  | _ when Ir_utils.is_irrefutable_nonbinding pat -> CannotFail (G.i Drop)
  | WildP -> assert false (* matched above *)
  | OptP p when Ir_utils.is_irrefutable_nonbinding p ->
      CanFail (fun fail_code ->
        Opt.is_some env ^^
        E.if0 G.nop fail_code)
  | OptP p ->
      let (set_x, get_x) = new_local env "opt_scrut" in
      CanFail (fun fail_code ->
        set_x ^^
        get_x ^^
        Opt.is_some env ^^
        E.if0
          ( get_x ^^
            Opt.project env ^^
            with_fail fail_code (fill_pat env ae p)
          )
          fail_code
      )
  | TagP ("", p) -> (* these only come from known_tag_pat *)
    if Ir_utils.is_irrefutable_nonbinding p
    then CannotFail (G.i Drop)
    else CannotFail (Variant.project env) ^^^ fill_pat env ae p
  | TagP (l, p) when Ir_utils.is_irrefutable_nonbinding p ->
      CanFail (fun fail_code ->
        Variant.test_is env l ^^
        E.if0 G.nop fail_code)
  | TagP (l, p) ->
      let (set_x, get_x) = new_local env "tag_scrut" in
      CanFail (fun fail_code ->
        set_x ^^
        get_x ^^
        Variant.test_is env l ^^
        E.if0
          ( get_x ^^
            Variant.project env ^^
            with_fail fail_code (fill_pat env ae p)
          )
          fail_code
      )
  | LitP l ->
      CanFail (fun fail_code ->
        compile_lit_pat env l ^^
        E.if0 G.nop fail_code)
  | VarP name ->
      CannotFail (Var.set_val_vanilla_from_stack env ae name)
  | TupP ps ->
      let (set_i, get_i) = new_local env "tup_scrut" in
      let rec go i = function
        | [] -> CannotFail G.nop
        | p::ps ->
          let code1 = fill_pat env ae p in
          let code2 = go (Int64.add i 1L) ps in
          CannotFail (get_i ^^ Tuple.load_n env i) ^^^ code1 ^^^ code2 in
      CannotFail set_i ^^^ go 0L ps
  | ObjP pfs ->
      let project = compile_load_field env pat.note in
      let (set_i, get_i) = new_local env "obj_scrut" in
      let rec go = function
        | [] -> CannotFail G.nop
        | {it={name; pat}; _}::pfs' ->
          let code1 = fill_pat env ae pat in
          let code2 = go pfs' in
          CannotFail (get_i ^^ project name) ^^^ code1 ^^^ code2 in
      CannotFail set_i ^^^ go pfs
  | AltP (p1, p2) ->
      let code1 = fill_pat env ae p1 in
      let code2 = fill_pat env ae p2 in
      let (set_i, get_i) = new_local env "alt_scrut" in
      CannotFail set_i ^^^
      orElse (CannotFail get_i ^^^ code1)
             (CannotFail get_i ^^^ code2)

and alloc_pat_local env ae pat =
  let d = Freevars.pat pat in
  AllocHow.M.fold (fun v typ ae ->
    let (ae1, _i) = VarEnv.add_direct_local env ae v SR.Vanilla typ
    in ae1
  ) d ae

and alloc_pat env ae how pat : VarEnv.t * G.t  =
  (fun (ae, code) -> (ae, G.with_region pat.at code)) @@
  let d = Freevars.pat pat in
  AllocHow.M.fold (fun v typ (ae, code0) ->
    let ae1, code1 = AllocHow.add_local env ae how v typ
    in (ae1, code0 ^^ code1)
  ) d (ae, G.nop)

and compile_pat_local env ae pat : VarEnv.t * patternCode =
  (* It returns:
     - the extended environment
     - the patternCode to do the pattern matching.
       This expects the  undestructed value is on top of the stack,
       consumes it, and fills the heap.
       If the pattern matches, execution continues (with nothing on the stack).
       If the pattern does not match, it fails (in the sense of PatCode.CanFail)
  *)
  let ae1 = alloc_pat_local env ae pat in
  let fill_code = fill_pat env ae1 pat in
  (ae1, fill_code)

(* Used for let patterns:
   If the pattern can consume its scrutinee in a better form than vanilla (e.g.
   unboxed tuple, unboxed 32/64), lets do that.
*)
and compile_unboxed_pat env ae how pat
  : VarEnv.t * G.t * G.t * SR.t option * G.t =
  (* It returns:
     - the extended environment
     - the code to allocate memory
     - the code to prepare the stack (e.g. push destination addresses)
       before the scrutinee is pushed
     - the desired stack rep. None means: Do not even push the scrutinee.
     - the code to do the pattern matching.
       This expects the undestructed value is on top of the stack,
       consumes it, and fills the heap
       If the pattern does not match, it traps with pattern failure
  *)
  let (ae1, alloc_code) = alloc_pat env ae how pat in
  let pre_code, sr, fill_code = match pat.it with
    (* Nothing to match: Do not even put something on the stack *)
    | WildP -> G.nop, None, G.nop
    (* Tuple patterns *)
    | TupP ps when List.length ps <> 1 ->
      G.nop,
      Some (SR.UnboxedTuple (List.length ps)),
      (* We have to fill the pattern in reverse order, to take things off the
         stack. This is only ok as long as patterns have no side effects.
      *)
      G.concat_mapi (fun i p -> orPatternFailure env (fill_pat env ae1 p)) (List.rev ps)
    (* Variable patterns *)
    | VarP name ->
      let pre_code, sr, code = Var.set_val env ae1 name in
      pre_code, Some sr, code
    (* The general case: Create a single value, match that. *)
    | _ ->
      G.nop,
      Some SR.Vanilla,
      orPatternFailure env (fill_pat env ae1 pat) in
  let pre_code = G.with_region pat.at pre_code in
  let fill_code = G.with_region pat.at fill_code in
  (ae1, alloc_code, pre_code, sr, fill_code)

and compile_dec env pre_ae how v2en dec : VarEnv.t * G.t * (VarEnv.t -> scope_wrap) =
  (fun (pre_ae, alloc_code, mk_code, wrap) ->
       G.(pre_ae, with_region dec.at alloc_code, fun ae body_code ->
          with_region dec.at (mk_code ae) ^^ wrap body_code)) @@

  match dec.it with
  (* A special case for public methods *)
  (* This relies on the fact that in the top-level mutually recursive group, no shadowing happens. *)
  | LetD ({it = VarP v; _}, e) when E.NameEnv.mem v v2en ->
    let exported_name = E.NameEnv.find v v2en in
    let (const, fill) = compile_const_exp env pre_ae e (Some exported_name) in
    let fi = match const with
      | Const.Message fi -> fi
      | _ -> assert false in
    let pre_ae1 = VarEnv.add_local_public_method pre_ae v (fi, exported_name) e.note.Note.typ in
    G.( pre_ae1, nop, (fun ae -> fill env ae; nop), unmodified)

  (* A special case for constant expressions *)
  | LetD (p, e) when e.note.Note.const ->
    (* constant expression matching with patterns is fully decidable *)
    if const_exp_matches_pat env pre_ae p e then (* not refuted *)
      let extend, fill = compile_const_dec env pre_ae dec in
      G.(extend pre_ae, nop, (fun ae -> fill env ae; nop), unmodified)
    else (* refuted *)
      (pre_ae, G.nop, (fun _ -> PatCode.patternFailTrap env), unmodified)

  | LetD (p, e) ->
    let (pre_ae1, alloc_code, pre_code, sr, fill_code) = compile_unboxed_pat env pre_ae how p in
    ( pre_ae1, alloc_code,
      (fun ae -> pre_code ^^ compile_exp_as_opt env ae sr e ^^ fill_code),
      unmodified
    )

  | VarD (name, content_typ, e) ->
    assert AllocHow.(match M.find_opt name how with
                     | Some (LocalMut _ | StoreHeap | StoreStatic) -> true
                     | _ -> false);
    let var_typ = Type.Mut content_typ in
    let pre_ae1, alloc_code = AllocHow.add_local env pre_ae how name var_typ in
    ( pre_ae1,
      alloc_code,
      (fun ae -> let pre_code, sr, code = Var.set_val env ae name in
                 pre_code ^^ compile_exp_as env ae sr e ^^ code),
      unmodified
    )

  | RefD (name, typ, { it = DotLE (e, n); _ }) ->
    let pre_ae1, alloc_code = AllocHow.add_local_for_alias env pre_ae how name typ in

    ( pre_ae1,
      alloc_code,
      (fun ae ->
        compile_exp_vanilla env ae e ^^
        Object.load_idx_raw env n ^^
        Var.capture_aliased_box env ae name),
      unmodified
    )
  | RefD _ -> assert false

and compile_decs_public env pre_ae decs v2en captured_in_body : VarEnv.t * scope_wrap =
  let how = AllocHow.decs pre_ae decs captured_in_body in
  let rec go pre_ae = function
    | []        -> (pre_ae, G.nop, fun _ -> unmodified)
    | [dec]     -> compile_dec env pre_ae how v2en dec
    | dec::decs ->
        let (pre_ae1, alloc_code1, mk_codeW1) = compile_dec env pre_ae how v2en dec in
        let (pre_ae2, alloc_code2, mk_codeW2) = go              pre_ae1 decs in
        ( pre_ae2,
          alloc_code1 ^^ alloc_code2,
          fun ae -> let codeW1 = mk_codeW1 ae in
                    let codeW2 = mk_codeW2 ae in
                    fun body_code -> codeW1 (codeW2 body_code)
        ) in
  let (ae1, alloc_code, mk_codeW) = go pre_ae decs in
  (ae1, fun body_code -> alloc_code ^^ mk_codeW ae1 body_code)

and compile_decs env ae decs captured_in_body : VarEnv.t * scope_wrap =
  compile_decs_public env ae decs E.NameEnv.empty captured_in_body

(* This compiles expressions determined to be const as per the analysis in
   ir_passes/const.ml. See there for more details.
*)
and compile_const_exp env pre_ae exp exported_name_opt : Const.v * (E.t -> VarEnv.t -> unit) =
  match exp.it with
  | FuncE (name, sort, control, typ_binds, args, res_tys, e) ->
    let fun_rhs =

      (* a few prims cannot be safely inlined *)
      let inlineable_prim = function
      | RetPrim -> false
      | BreakPrim _ -> false
      | ThrowPrim -> fatal "internal error: left-over ThrowPrim"
      | _ -> true in

      match sort, control, typ_binds, e.it with
      (* Special cases for prim-wrapping functions *)

      | Type.Local, Type.Returns, [], PrimE (prim, prim_args) when
          inlineable_prim prim &&
          List.length args = List.length prim_args &&
          List.for_all2 (fun p a -> a.it = VarE (Const, p.it)) args prim_args ->
        Const.PrimWrapper prim
      | _, _, _, _ -> Const.Complicated
    in
    let return_tys = match control with
      | Type.Returns -> res_tys
      | Type.Replies -> []
      | Type.Promises -> assert false in
    let mk_body env ae =
      List.iter (fun v ->
        if not (VarEnv.NameEnv.mem v ae.VarEnv.vars)
        then fatal "internal error: const \"%s\": captures \"%s\", not found in static environment\n" name v
      ) (Freevars.M.keys (Freevars.exp e));
      compile_exp_as env ae (StackRep.of_arity (List.length return_tys)) e in
    FuncDec.closed env sort control name args mk_body fun_rhs return_tys exp.at exported_name_opt
  | BlockE (decs, e) ->
    let (extend, fill1) = compile_const_decs env pre_ae decs in
    let ae' = extend pre_ae in
    let (c, fill2) = compile_const_exp env ae' e None in
    (c, fun env ae ->
      let ae' = extend ae in
      fill1 env ae';
      fill2 env ae')
  | VarE (_, v) ->
    let c =
      match VarEnv.lookup_var pre_ae v with
      | Some (VarEnv.Const c) -> c
      | _ -> fatal "compile_const_exp/VarE: \"%s\" not found" v
    in
    (c, fun _ _ -> ())
  | NewObjE (Type.(Object | Module | Memory), fs, _) ->
    let static_fs = List.map (fun f ->
          let st =
            match VarEnv.lookup_var pre_ae f.it.var with
            | Some (VarEnv.Const c) -> c
            | _ -> fatal "compile_const_exp/ObjE: \"%s\" not found" f.it.var
          in f.it.name, st) fs
    in
    (Const.Obj static_fs), fun _ _ -> ()
  | PrimE (DotPrim name, [e]) ->
    let (object_ct, fill) = compile_const_exp env pre_ae e None in
    let fs = match object_ct with
      | Const.Obj fs -> fs
      | _ -> fatal "compile_const_exp/DotE: not a static object" in
    let member_ct = List.assoc name fs in
    (member_ct, fill)
  | PrimE (ProjPrim i, [e]) ->
    let (object_ct, fill) = compile_const_exp env pre_ae e None in
    let cs = match object_ct with
      | Const.Tuple cs -> cs
      | _ -> fatal "compile_const_exp/ProjE: not a static tuple" in
    (List.nth cs i, fill)
  | LitE l -> Const.(Lit (const_lit_of_lit l)), (fun _ _ -> ())
  | PrimE (TupPrim, []) -> Const.Unit, (fun _ _ -> ())
  | PrimE (ArrayPrim (Const, _), es) ->
    let (cs, fills) = List.split (List.map (fun e -> compile_const_exp env pre_ae e None) es) in
    (Const.Array cs),
    (fun env ae -> List.iter (fun fill -> fill env ae) fills)
  | PrimE (TupPrim, es) ->
    let (cs, fills) = List.split (List.map (fun e -> compile_const_exp env pre_ae e None) es) in
    (Const.Tuple cs),
    (fun env ae -> List.iter (fun fill -> fill env ae) fills)
  | PrimE (TagPrim i, [e]) ->
    let (arg_ct, fill) = compile_const_exp env pre_ae e None in
    (Const.Tag (i, arg_ct)),
    fill
  | PrimE (OptPrim, [e]) ->
    let (arg_ct, fill) = compile_const_exp env pre_ae e None in
    (Const.Opt arg_ct),
    fill

  | _ -> assert false

and compile_const_decs env pre_ae decs : (VarEnv.t -> VarEnv.t) * (E.t -> VarEnv.t -> unit) =
  let rec go pre_ae = function
    | []          -> (fun ae -> ae), (fun _ _ -> ())
    | [dec]       -> compile_const_dec env pre_ae dec
    | (dec::decs) ->
        let (extend1, fill1) = compile_const_dec env pre_ae dec in
        let pre_ae1 = extend1 pre_ae in
        let (extend2, fill2) = go                    pre_ae1 decs in
        (fun ae -> extend2 (extend1 ae)),
        (fun env ae -> fill1 env ae; fill2 env ae) in
  go pre_ae decs

and const_exp_matches_pat env ae pat exp : bool =
  assert exp.note.Note.const;
  let c, _ = compile_const_exp env ae exp None in
  match destruct_const_pat VarEnv.empty_ae pat c with Some _ -> true | _ -> false

and destruct_const_pat ae pat const : VarEnv.t option = match pat.it with
  | WildP -> Some ae
  | VarP v -> Some (VarEnv.add_local_const ae v const pat.note)
  | ObjP pfs ->
    let fs = match const with Const.Obj fs -> fs | _ -> assert false in
    List.fold_left (fun ae (pf : pat_field) ->
      match ae, List.find_opt (fun (n, _) -> pf.it.name = n) fs with
      | None, _ -> None
      | Some ae, Some (_, c) -> destruct_const_pat ae pf.it.pat c
      | _, None -> assert false
    ) (Some ae) pfs
  | AltP (p1, p2) ->
    let l = destruct_const_pat ae p1 const in
    if l = None then destruct_const_pat ae p2 const
    else l
  | TupP ps ->
    let cs = match const with Const.Tuple cs -> cs | Const.Unit -> [] | _ -> assert false in
    let go ae p c = match ae with
      | Some ae -> destruct_const_pat ae p c
      | _ -> None in
    List.fold_left2 go (Some ae) ps cs
  | LitP lp ->
    begin match const with
    | Const.Lit lc when Const.lit_eq (const_lit_of_lit lp) lc -> Some ae
    | _ -> None
    end
  | OptP p ->
    begin match const with
      | Const.Opt c -> destruct_const_pat ae p c
      | Const.(Lit Null) -> None
      | _ -> assert false
    end
  | TagP (i, p) ->
     match const with
     | Const.Tag (ic, c) when i = ic -> destruct_const_pat ae p c
     | Const.Tag _ -> None
     | _ -> assert false

and compile_const_dec env pre_ae dec : (VarEnv.t -> VarEnv.t) * (E.t -> VarEnv.t -> unit) =
  (* This returns a _function_ to extend the VarEnv, instead of doing it, because
  it needs to be extended twice: Once during the pass that gets the outer, static values
  (no forward references), and then to implement the `fill`, which compiles the bodies
  of functions (may contain forward references.) *)
  match dec.it with
  (* This should only contain constants (cf. is_const_exp) *)
  | LetD (p, e) ->
    let (const, fill) = compile_const_exp env pre_ae e None in
    (fun ae -> match destruct_const_pat ae p const with Some ae -> ae | _ -> assert false),
    (fun env ae -> fill env ae)
  | VarD _ | RefD _ -> fatal "compile_const_dec: Unexpected VarD/RefD"

and compile_init_func mod_env ((cu, flavor) : Ir.prog) =
  assert (not flavor.has_typ_field);
  assert (not flavor.has_poly_eq);
  assert (not flavor.has_show);
  assert (not flavor.has_await);
  assert (not flavor.has_async_typ);
  match cu with
  | LibU _ -> fatal "compile_start_func: Cannot compile library"
  | ProgU ds ->
    Func.define_built_in mod_env "init" [] [] (fun env ->
      let ae, codeW = compile_decs env VarEnv.empty_ae ds Freevars.S.empty in
      (* Register the Blob deserialization dedup function *)
      Internals.register_dedup env ae;
      codeW G.nop
    )
  | ActorU (as_opt, ds, fs, up, t) ->
    main_actor as_opt mod_env ds fs up

and export_actor_field env  ae (f : Ir.field) =
  (* A public actor field is guaranteed to be compiled as a PublicMethod *)
  let fi =
    match VarEnv.lookup_var ae f.it.var with
    | Some (VarEnv.PublicMethod (fi, _)) -> fi
    | _ -> assert false in

  E.add_export env (nr {
    name = Lib.Utf8.decode (match E.mode env with
      | Flags.ICMode | Flags.RefMode ->
        Mo_types.Type.(
        match normalize f.note with
        |  Func(Shared sort,_,_,_,_) ->
           (match sort with
            | Write -> "canister_update " ^ f.it.name
            | Query -> "canister_query " ^ f.it.name
            | Composite -> "canister_composite_query " ^ f.it.name
           )
        | _ -> assert false)
      | _ -> assert false);
    edesc = nr (FuncExport (nr fi))
  })

(* Main actor *)
and main_actor as_opt mod_env ds fs up =
  let stable_actor_type = up.stable_type in
  let build_stable_actor = up.stable_record in
  IncrementalGraphStabilization.define_methods mod_env stable_actor_type;

  (* Export metadata *)
  mod_env.E.stable_types := metadata "motoko:stable-types" up.meta.sig_;
  mod_env.E.service := metadata "candid:service" up.meta.candid.service;
  mod_env.E.args := metadata "candid:args" up.meta.candid.args;

  Func.define_built_in mod_env IC.initialize_main_actor_function_name [] [] (fun env ->
    let ae0 = VarEnv.empty_ae in
    let captured = Freevars.captured_vars (Freevars.actor ds fs up) in
    (* Add any params to the environment *)
    (* Captured ones need to go into static memory, the rest into locals *)
    let args = match as_opt with None -> [] | Some as_ -> as_ in
    let arg_list = List.map (fun a -> (a.it, a.note)) args in
    let arg_names = List.map (fun a -> a.it) args in
    let arg_tys = List.map (fun a -> a.note) args in
    let as_local n = not (Freevars.S.mem n captured) in
    let ae1 = VarEnv.add_arguments env ae0 as_local arg_list in

    (* Reverse the fs, to a map from variable to exported name *)
    let v2en = E.NameEnv.from_list (List.map (fun f -> (f.it.var, f.it.name)) fs) in

    (* Compile the declarations *)
    let ae2, decls_codeW = compile_decs_public env ae1 ds v2en
        Freevars.(captured_vars (system up))
    in
    (* Register the Blob deserialization dedup function *)
    Internals.register_dedup env ae2;
    (* Export the public functions *)
    List.iter (export_actor_field env ae2) fs;

    (* Export upgrade hooks *)
    Func.define_built_in env "pre_exp" [] [] (fun env ->
      compile_exp_as env ae2 SR.unit up.preupgrade);
    Func.define_built_in env "post_exp" [] [] (fun env ->
      compile_exp_as env ae2 SR.unit up.postupgrade);
    IC.export_upgrade_methods env;

    (* Export heartbeat (but only when required) *)
    begin match up.heartbeat.it with
     | Ir.PrimE (Ir.TupPrim, []) -> ()
     | _ ->
       Func.define_built_in env "heartbeat_exp" [] [] (fun env ->
         compile_exp_as env ae2 SR.unit up.heartbeat);
       IC.export_heartbeat env;
    end;

    (* Export timer (but only when required) *)
    begin match up.timer.it with
     | Ir.PrimE (Ir.TupPrim, []) -> ()
     | _ ->
       Func.define_built_in env "timer_exp" [] [] (fun env ->
         compile_exp_as env ae2 SR.unit up.timer);
       IC.export_timer env;
    end;

    (* Export inspect (but only when required) *)
    begin match up.inspect.it with
     | Ir.PrimE (Ir.TupPrim, []) -> ()
     | _ ->
       Func.define_built_in env "inspect_exp" [] [] (fun env ->
         compile_exp_as env ae2 SR.unit up.inspect);
       IC.export_inspect env;
    end;

    (* Export low memory hook (but only when required) *)
    begin match up.low_memory.it with
    | Ir.PrimE (Ir.TupPrim, []) -> ()
    | _ ->
      Func.define_built_in env "low_memory_exp" [] [] (fun env ->
        compile_exp_as env ae2 SR.unit up.low_memory);
      IC.export_low_memory env;
    end;

    (* Helper function to build the stable actor wrapper *)
    Func.define_built_in mod_env IC.get_actor_to_persist_function_name [] [I64Type] (fun env ->
      compile_exp_as env ae2 SR.Vanilla build_stable_actor
    );

    (* Deserialize the init arguments *)
    begin match as_opt with
      | None
      | Some [] ->
        (* Liberally accept empty as well as unit argument *)
        assert (arg_tys = []);
        Persistence.get_init_message_payload env ^^
        Blob.len env ^^
        compile_eq_const 0L ^^
        E.if0
          G.nop
          begin
            (* Only validate the message payload. *)
            Persistence.get_init_message_payload env ^^
            Bool.lit false ^^ (* cannot recover *)
            Serialization.deserialize_from_blob false env arg_tys
          end
      | Some (_ :: _) ->
        Persistence.get_init_message_payload env ^^
        Bool.lit false ^^ (* cannot recover *)
        Serialization.deserialize_from_blob false env arg_tys ^^
        G.concat_map (Var.set_val_vanilla_from_stack env ae1) (List.rev arg_names)
    end ^^
    begin
      if up.timer.at <> no_region then
        (* initiate a timer pulse *)
        compile_unboxed_one ^^
        IC.system_call env "global_timer_set" ^^
        G.i Drop
      else
        G.nop
    end ^^

    decls_codeW G.nop
  );

  Func.define_built_in mod_env "init" [] [] (fun env ->
    IC.init_globals env ^^
    (* Save the init message payload for later deserializtion. *)
    IC.arg_data env ^^
    Persistence.set_init_message_payload env ^^
    Persistence.initialize env stable_actor_type
  )

and metadata name value =
  if List.mem name !Flags.omit_metadata_names then None
  else Some (
           List.mem name !Flags.public_metadata_names,
           value)

and conclude_module env set_serialization_globals start_fi_o =

  RTS_Exports.system_exports env;

  FuncDec.export_async_method env;
  FuncDec.export_gc_trigger_method env;
  FuncDec.export_stabilization_limits env;

  (* See Note [Candid subtype checks] *)
  Serialization.create_global_type_descriptor env set_serialization_globals;

  (* declare before building GC *)

  (* add beginning-of-heap pointer, may be changed by linker *)
  (* needs to happen here now that we know the size of static memory *)
  let set_heap_base = E.add_global64_delayed env "__heap_base" Immutable in
  E.export_global env "__heap_base";

  Heap.register env;
  IC.register env;

  let dynamic_heap_start = Lifecycle.end_ () in
  set_heap_base dynamic_heap_start;

  (* Wrap the start function with the RTS initialization *)
  let rts_start_fi = E.add_fun env "rts_start" (Func.of_body env [] [] (fun env1 ->
    E.call_import env "rts" ("initialize_incremental_gc") ^^
    GCRoots.register_static_variables env ^^
    match start_fi_o with
    | Some fi ->
      G.i (Call fi)
    | None ->
      Lifecycle.set env Lifecycle.PreInit
  )) in

  IC.default_exports env;

  let func_imports, ni, remapping = E.finalize_func_imports env in

  let initial_memory_pages = Int64.(add (div dynamic_heap_start page_size) 1L) in
  let memories = E.get_memories env initial_memory_pages in

  let funcs = E.get_funcs env in

  let datas = List.map (fun (dinit) -> nr {
    dinit;
    dmode = (nr Wasm_exts.Ast.Passive);
    }) (E.get_data_segments env) in

  let elems = List.map (fun (fi, fp) -> nr {
    index = nr 0l;
    offset = nr (G.to_instr_list (compile_unboxed_const (Int64.of_int32 fp)));
    init = [ nr fi ];
    }) (E.get_elems env) in

  let table_sz = E.get_end_of_table env in

  let module_ = rename_funcs remapping {
      types = List.map nr (E.get_types env);
      funcs = List.map (fun (f,_,_) -> f) funcs;
      tables = [ nr { ttype = HugeTableType (Int64.{ min = of_int32 table_sz; max = Some (of_int32 table_sz) }, FuncRefType) } ];
      elems;
      start = Some (nr rts_start_fi);
      globals = E.get_globals env;
      memories;
      imports = func_imports;
      exports = E.get_exports env;
      datas
    } in

  let emodule =
    let open Wasm_exts.CustomModule in
    { module_;
      dylink0 = [];
      name = { empty_name_section with
        function_names = List.mapi (fun i (f,n,_) -> Int32.(add ni (of_int i), n)) funcs;
        locals_names = List.mapi (fun i (f,_,ln) -> Int32.(add ni (of_int i), ln)) funcs;
      };
      motoko = {
        labels = E.get_labs env;
        stable_types = !(env.E.stable_types);
        compiler = metadata "motoko:compiler" (Lib.Option.get Source_id.release Source_id.id);
      };
      enhanced_orthogonal_persistence = Some (false, "64-bit, layout version 1");
      candid = {
        args = !(env.E.args);
        service = !(env.E.service);
      };
      source_mapping_url = None;
      wasm_features = E.get_features env;
    } in

  (* For debugging *)
  if !Flags.verbose then E.object_pool_report env;

  match E.get_rts env with
  | None -> emodule
  | Some rts -> Linking.LinkModule.link emodule "rts" rts

let compile mode rts (prog : Ir.prog) : Wasm_exts.CustomModule.extended_module =
  (* Enhanced orthogonal persistence requires a fixed layout. *)
  assert !Flags.rtti; (* Use precise tagging for graph copy. *)
  assert (!Flags.gc_strategy = Flags.Incremental); (* Define heap layout with the incremental GC. *)
  let env = E.mk_global mode rts IC.trap_with in

  IC.register_globals env;
  Stack.register_globals env;
  GC.register_globals env;
  StableMem.register_globals env;
  Serialization.Registers.register_globals env;
  Serialization.Registers.define_idl_limit_check env;
  IncrementalGraphStabilization.register_globals env;
  Persistence.register_globals env;

  (* See Note [Candid subtype checks] *)
  let set_serialization_globals = Serialization.register_delayed_globals env in
  Serialization.reserve_global_type_descriptor env;

  IC.system_imports env;
  RTS.system_imports env;

  compile_init_func env prog;
  let start_fi_o = match E.mode env with
    | Flags.ICMode | Flags.RefMode ->
      IC.export_init env;
      None
    | Flags.WASIMode ->
      IC.export_wasi_start env;
      None
    | Flags.WasmMode ->
      Some (nr (E.built_in env "init"))
  in
  conclude_module env set_serialization_globals start_fi_o
