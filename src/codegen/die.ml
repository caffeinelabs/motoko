open Mo_types
open Wasm_exts.Dwarf5
open Meta

(* Note [Low_pc, High_pc, Ranges are special]
   ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The DWARF attributes `Low_pc`, `High_pc` and `Ranges` carry information
about the Wasm bytecode's layout in the emitted Code section for the
compilation unit. The information is not available here, so these
attributes have no payload at this side. Instead it is filled in
in a tag-dependent manner by `customModuleEncode.ml`. For LexicalBlock
the Low_pc and High_pc attributes are managed entirely by the emitter.
 *)

type dw_AT = Producer of string
           | Language of int
           | Name of string
           | Stmt_list of int
           | Comp_dir of string
           | Use_UTF8 of bool
           | Low_pc | High_pc | Ranges (* see Note [Low_pc, High_pc, Ranges are special] *)
           | Addr_base of int
           | Decl_file of string
           | Decl_line of int
           | Decl_column of int
           | Prototyped of bool
           | External of bool
           | Byte_size of int
           | Bit_size of int
           | Data_bit_offset of int
           | Discr of int (* reference *)
           | Const_value of int
           | Discr_value of int
           | Artificial of bool
           | TypeRef of int (* reference *)
           | TypePromise of int Lib.Promise.t (* reference *)
           | Encoding of int
           | Location of int list
           | DataMemberLocation of int

(* DWARF tags *)

type dw_TAG =
  | Compile_unit of string * string                            (* compilation directory, file name *)
  | Subprogram of string * Type.typ list * Source.pos          (* name, return types, location *)
  | LexicalBlock of Source.pos
  | Formal_parameter of (string * Source.pos * Type.typ * int) (* name, location, type, Wasm slot *)
  | Variable of (string * Source.pos * Type.typ * int)         (* name, location, type, Wasm slot *)
  | Type of Type.typ
  | Typedef of string * Type.typ
  (*| Member*)
  | Variant_part
  | Variant

(* DWARF high-level structures *)

let dw_attr' : dw_AT -> die =
  let bool b = if b then 1 else 0 in
  function
  | Producer p -> StringAttribute (dw_AT_producer, p)
  | Language l -> IntAttribute (dw_AT_language, l)
  | Name n -> StringAttribute (dw_AT_name, n)
  | Stmt_list l -> IntAttribute (dw_AT_stmt_list, l)
  | Comp_dir d -> StringAttribute (dw_AT_comp_dir, d)
  | Use_UTF8 b -> IntAttribute (dw_AT_use_UTF8, bool b)
  | Addr_base b -> IntAttribute (dw_AT_addr_base, b)
  | Low_pc -> OffsetAttribute dw_AT_low_pc
  | High_pc -> OffsetAttribute dw_AT_high_pc
  | Ranges -> OffsetAttribute dw_AT_ranges  (* see Note [Low_pc, High_pc, Ranges are special] *)
  | Decl_file f -> StringAttribute (dw_AT_decl_file, f)
  | Decl_line l -> IntAttribute (dw_AT_decl_line, l)
  | Decl_column c -> IntAttribute (dw_AT_decl_column, c)
  | Prototyped b -> IntAttribute (dw_AT_prototyped, bool b)
  | External b -> IntAttribute (dw_AT_external, bool b)
  | Byte_size s -> IntAttribute (dw_AT_byte_size, s)
  | Bit_size s -> IntAttribute (dw_AT_bit_size, s)
  | Data_bit_offset o -> IntAttribute (dw_AT_data_bit_offset, o)
  | Artificial b -> IntAttribute (dw_AT_artificial, bool b)
  | Discr r -> IntAttribute (dw_AT_discr, r)
  | TypeRef r -> IntAttribute (dw_AT_type, r)
  | TypePromise p ->
    (* See Note [placeholder promises for typedefs] *)
    IntAttribute (dw_AT_type, Wasm_exts.CustomModuleEncode.promise_reference_slot p)
  | Encoding e -> IntAttribute (dw_AT_encoding, e)
  | Discr_value v -> IntAttribute (dw_AT_discr_value, v)
  | Const_value v -> IntAttribute (dw_AT_const_value, v)
  | DataMemberLocation offs -> IntAttribute (dw_AT_data_member_location, offs)
  | Location ops ->
    let string_of_ops ops =
      let open Buffer in
      let buf = create 16 in
      let rec stash = function
        | i when i >= 0 -> assert (i < 0x100); add_char buf (Char.chr i)
        | i when -i < 128 -> stash (-i) (* ULEB128 byte *)
        | i -> (* needs ULEB128 chopping *)
          let i = -i in
          stash (i land 0x7F lor 0x80);
          stash (- (i lsr 7)) in
      List.iter stash ops;
      contents buf in
    StringAttribute (dw_AT_location, string_of_ops ops)

let dw_attr at : die list = [dw_attr' at]

let dw_attrs = List.map dw_attr'

let unreferencable_tag tag attrs =
  Tag (None, tag, attrs)


let with_referencable_tag f tag attrs : die * int =
  let refslot = Wasm_exts.CustomModuleEncode.allocate_reference_slot () in
  f refslot;
  Tag (Some refslot, tag, attrs),
  refslot

let with_referencable_tags f tag attrs : die list * int =
  let t, r = with_referencable_tag f tag attrs in
  [t], r

let with_closed_referencable_tags f tag attrs : die list * int =
  let t, r = with_referencable_tag f tag attrs in
  [Grouped [TagClose; t]], r

let referencable_tag = with_referencable_tag ignore

let autoclose_unreferencable_tag tag attrs =
  Grouped [TagClose; unreferencable_tag tag attrs]

let autoclose_referencable_tag tag attrs =
  let t, r = referencable_tag tag attrs in
  Grouped [TagClose; t], r


let obvious_prim_of_con c ty : Type.prim option =
  match Type.normalize ty with
  | Type.Prim p ->
    if Arrange_type.(prim p = con c) then Some p else None
  | _ -> None

let is_enum =
  let no_payload = function
    | Type.{typ = Tup []; _} -> true
    | _ -> false in
  List.for_all no_payload

(* Mutable state for the already referencable type DIEs *)
let any_type = ref None
module TypedefRefs = Map.Make (struct type t = Type.kind Cons.t let compare = compare end)
let dw_typedefs = ref TypedefRefs.empty
module PrimRefs = Map.Make (struct type t = Type.prim let compare = compare end)
let dw_prims = ref PrimRefs.empty
module EnumRefs = Map.Make (struct type t = string list let compare = compare end)
let dw_enums = ref EnumRefs.empty
module OptionRefs = Map.Make (struct type t = int let compare = compare end)
let dw_options = ref OptionRefs.empty
module VariantRefs = Map.Make (struct type t = (string * int) list let compare = compare end)
let dw_variants = ref VariantRefs.empty
module ObjectRefs = Map.Make (struct type t = (string * int) list let compare = compare end)
let dw_objects = ref ObjectRefs.empty
module TupleRefs = Map.Make (struct type t = int list let compare = compare end)
let dw_tuples = ref TupleRefs.empty

(* Factory for referencable type DIEs *)
let rec type_ref : Type.typ -> die list * int =
  let open Type in
  function
  | Any ->
    begin match !any_type with
    | Some r -> [], r
    | None ->
      let add r = any_type := Some r in
      with_referencable_tags add dw_TAG_base_type
          (dw_attrs [Name "Any"; Bit_size 0; Data_bit_offset 0; Encoding dw_ATE_address])
    end
  | Prim pr -> prim_type_ref pr
  | Variant vs when is_enum vs -> enum vs
  | Variant vs -> variant vs
  | Obj (Object, fs, _) -> object_ fs
  | Tup cs -> tuple cs
  | Con (c, _) as ty ->
    begin match obvious_prim_of_con c ty with
    | Some p -> type_ref (Prim p)
    | None -> typedef_ref c ty
    end
  | Opt inner ->
    let prereq, selector = type_ref inner in
    let opt, r = option_instance selector in
    prereq @ opt, r
  | typ -> (*Printf.printf "Cannot type typ: %s\n" (Wasm.Sexpr.to_string 80 (Arrange_type.typ typ));*) type_ref Any (* FIXME: assert false *)

and typedef_ref c ty : die list * int =
  match TypedefRefs.find_opt c !dw_typedefs with
  | Some r -> ([], r)
  | None ->
    let add r = dw_typedefs := TypedefRefs.add c r !dw_typedefs in
    (* See Note [placeholder promises for typedefs] *)
    let p = Lib.Promise.make () in
    let name = match Arrange_type.con c with | Wasm.Sexpr.Atom n -> n | _ -> assert false in
    let typedef_tag, typedef_ref = with_referencable_tag add dw_TAG_typedef (dw_attrs [Name name; TypePromise p]) in
    let ds, reference = type_ref (Type.normalize ty) in
    Lib.Promise.fulfill p reference;
    typedef_tag :: ds, typedef_ref

and prim_type_ref (prim : Type.prim) : die list * int =
  match PrimRefs.find_opt prim !dw_prims with
  | Some r -> [], r
  | None ->
    let name = Name (Type.string_of_prim prim) in
    let add r = dw_prims := PrimRefs.add prim r !dw_prims in
    let ds, r =
      match prim with
      | Type.Bool ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 1; Data_bit_offset 1; Encoding dw_ATE_boolean])
      | Type.Char ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 29; Data_bit_offset 8; Encoding dw_ATE_UTF])
      | Type.(Int | Nat) ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 64; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_signed])
      | Type.Text -> (* FIXME: should be dynamic, like Any *)
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 64; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_UTF])
      | Type.(Int8|Int16|Int32) ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 32; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_signed])
      | Type.(Nat8|Nat16|Nat32) ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 32; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_unsigned])
      | Type.Int64 ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 64; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_signed])
      | Type.Nat64 ->
        with_referencable_tags add dw_TAG_base_type
          (dw_attrs [name; Bit_size 64; Data_bit_offset 0(*FIXME: for now*); Encoding dw_ATE_unsigned])
      | ty -> (*Printf.eprintf "Cannot type: %s\n" (Wasm.Sexpr.to_string 80 (Arrange_type.prim prim));*) type_ref Type.Any (* FIXME, this is "Any" for now *)
(* | _ -> assert false (* TODO *)*)
    in
    ds, r

and enum vnts : die list * int =
  let selectors = List.map (fun Type.{lab; _} -> lab) vnts in
  match EnumRefs.find_opt selectors !dw_enums with
  | Some r -> [], r
  | None ->
    let add r = dw_enums := EnumRefs.add selectors r !dw_enums in
    let enumerator name =
        let hash = Int32.to_int (Mo_types.Hash.hash name) in
        unreferencable_tag dw_TAG_enumerator (dw_attrs [Name name; Const_value hash]) in
    (*  enumeration_type, useful only with location expression *)
    with_closed_referencable_tags add
       dw_TAG_enumeration_type
       (dw_attr' (Artificial true) :: List.map enumerator selectors)

and option_instance key : die list * int =
  (* TODO: make this with DW_TAG_template_alias? ... lldb-10 is not ready yet *)
  let open Wasm_exts.Abbreviation in
  match OptionRefs.find_opt key !dw_options with
  | Some r -> [], r
  | None ->
    let add r = dw_options := OptionRefs.add key r !dw_options in
    let prereq name(*WAT?*) : die * die =
      let overlay_die, overlay_ref =
        autoclose_referencable_tag dw_TAG_structure_type
          (unreferencable_tag dw_TAG_member_In_variant
             (dw_attrs [Name "?"; TypeRef key; DataMemberLocation 4]) ::
           dw_attrs [Name name; Byte_size 8 (*; Artificial *)]) in
      overlay_die,
      unreferencable_tag dw_TAG_member_In_variant
        (dw_attrs [Name name; TypeRef overlay_ref; DataMemberLocation 4]) in
    (* make sure all prerequisite types are around *)
    let overlays = List.map prereq ["FIXME:none"; "FIXME:some"] in
    (* struct_type, assumes location points at heap tag -- NO! FIXME *)
    let discr_tag, discr_ref =
      referencable_tag dw_TAG_member_Variant_mark
        (dw_attrs [Artificial true; Byte_size 4; DataMemberLocation 0]) in
    let summand ((name, discr), member) : die =
      autoclose_unreferencable_tag dw_TAG_variant_Named
        (member :: dw_attrs [Name name; Discr_value discr]) in
    let internal_struct, struct_ref =
      with_closed_referencable_tags add dw_TAG_structure_type
        (discr_tag ::
         autoclose_unreferencable_tag dw_TAG_variant_part
           (dw_attr' (Discr discr_ref) ::
            List.map summand (List.map2 (fun nd (_, mem) -> nd, mem) ["FIXME:none", 0x0; "FIXME:some", 0x8] overlays)) ::
         dw_attrs [Name "OPTION"; Byte_size 8]) in
    List.map fst overlays @ internal_struct,
    struct_ref

and variant vnts : die list * int =
  let open Wasm_exts.Abbreviation in
  let selectors = List.map (fun Type.{lab; typ; _} -> lab, typ, type_ref typ) vnts in
  (* make sure all prerequisite types are around *)
  let prereqs = List.concat_map (fun (_, _, (dw, _)) -> dw) selectors in
  let key = List.map (fun (name, _, (_, reference)) -> name, reference) selectors in
  match VariantRefs.find_opt key !dw_variants with
  | Some r -> prereqs, r
  | None ->
    let add r = dw_variants := VariantRefs.add key r !dw_variants in
    let prereq (name, typ, _) : die list * die =
      let (payload_pre, payload_mem) : die list * die list =
        match typ with
        | Type.Tup [] -> [], []
        | _ ->
          let ds, r = type_ref typ in
          ds,
          [unreferencable_tag dw_TAG_member_In_variant
             (dw_attrs [Name ("#" ^ name); TypeRef r; DataMemberLocation 8])] in
      let overlay_ds, (overlay_die, overlay_ref) =
        payload_pre,
        autoclose_referencable_tag dw_TAG_structure_type
          (dw_attrs [Name name; Byte_size 12 (*; Artificial *)] @ payload_mem) in
      overlay_ds @ [overlay_die],
      unreferencable_tag dw_TAG_member_In_variant
        (dw_attrs [Name name; TypeRef overlay_ref; DataMemberLocation 8]) in
    (* make sure all artificial overlay types are around *)
    let overlays = List.map prereq selectors in
    let discr_tag, discr_ref =
      referencable_tag dw_TAG_member_Variant_mark
        (dw_attrs [Artificial true; Byte_size 4; DataMemberLocation 4]) in
    (* struct_type, assumes location points at heap tag *)
    let summand (name, member) : die =
      let hash = Int32.to_int (Mo_types.Hash.hash name) in
      autoclose_unreferencable_tag dw_TAG_variant_Named
        (member :: dw_attrs [Name name; Discr_value hash]) in
    let internal_struct, struct_ref =
      with_closed_referencable_tags add dw_TAG_structure_type
        (autoclose_unreferencable_tag dw_TAG_member_Tag_mark
           (dw_attrs [Artificial true; Byte_size 4]) ::
        discr_tag ::
        (autoclose_unreferencable_tag
           dw_TAG_variant_part (
             List.map summand (List.map2 (fun (name, _, _) (_, mem) -> name, mem) selectors overlays) @
             dw_attrs [Discr discr_ref])) ::
        dw_attrs [Name "VARIANT"; Byte_size 8]) in
    prereqs @ List.concat_map fst overlays @ internal_struct, struct_ref

and object_ fs : die list * int =
  let open List in
  let open Wasm_exts.Abbreviation in
  let selectors = map (fun Type.{lab; typ; _} -> lab, type_ref typ) fs in
  (* make sure all prerequisite types are around *)
  let prereqs = concat_map (fun (_, (ds, _)) -> ds) selectors in
  let key = map (fun (name, (_, reference)) -> name, reference) selectors in
  match ObjectRefs.find_opt key !dw_objects with
  | Some r -> prereqs, r
  | None ->
    let add r = dw_objects := ObjectRefs.add key r !dw_objects in
    let ds, r =
      let field (name, (_, r)) : die =
        let _hash = Lib.Uint32.to_int (Idllib.IdlHash.idl_hash name) in (* TODO *)
        unreferencable_tag dw_TAG_member_Word_sized_typed
          (dw_attrs [Name name; TypeRef r; Byte_size 4 (*; Location search *)]) in
      (* reference to structure_type *)
      with_closed_referencable_tags add dw_TAG_structure_type
          (dw_attrs [Name "@obj"; Byte_size (4 * length selectors)] @ map field selectors) in
    prereqs @ ds, r

and tuple ts : die list * int =
  let open List in
  let open Wasm_exts.Abbreviation in
  let field_types_refs = map type_ref ts in
  let field_refs = map snd field_types_refs in
  let prereqs = concat_map fst field_types_refs in
  match TupleRefs.find_opt field_refs !dw_tuples with
  | Some r -> prereqs, r
  | None ->
    let add r = dw_tuples := TupleRefs.add field_refs r !dw_tuples in
    let ds, r =
      let field index (_, r) =
        unreferencable_tag dw_TAG_member_Word_sized_typed
          (dw_attrs [Name (Printf.sprintf ".%d" index); TypeRef r; Byte_size 4]) in
      with_closed_referencable_tags add dw_TAG_structure_type
        (dw_attrs [Name "@tup"; Byte_size 4] @ mapi field field_types_refs) in
    prereqs @ ds, r



(* Location expressions for DWARF
   These are instruction sequences that direct the debugger
   at where the data resides.
 *)
let rec loc slot =
  let unskew, past_tag = 1, 4 in
  let open Type in
  function (* See Note [locations for types] *)
  | Type.Variant vs when is_enum vs ->
    Location.local slot [ dw_OP_plus_uconst; unskew + past_tag; dw_OP_deref; dw_OP_stack_value ]
  | Type.Variant _ -> Location.local slot [ dw_OP_plus_uconst; unskew ]
  | Prim Text -> Location.local slot [ dw_OP_plus_uconst; unskew; dw_OP_stack_value ]
  | Prim Char -> Location.local slot [ dw_OP_lit8; dw_OP_shr; dw_OP_stack_value ]
  | Prim Bool -> Location.local slot [ dw_OP_lit1; dw_OP_shr; dw_OP_stack_value ]
  | Prim Int8 -> Location.local slot [ dw_OP_lit24; dw_OP_shra; dw_OP_stack_value ]
  | Prim Nat8 -> Location.local slot [ dw_OP_lit24; dw_OP_shr; dw_OP_stack_value ]
  | Prim Int16 -> Location.local slot [ dw_OP_lit16; dw_OP_shra; dw_OP_stack_value ]
  | Prim Nat16 -> Location.local slot [ dw_OP_lit16; dw_OP_shr; dw_OP_stack_value ]
  | Prim Int32 -> Location.local slot [ dw_OP_dup; dw_OP_lit1; dw_OP_and; dw_OP_bra; 5; 0;
                                        dw_OP_lit1; dw_OP_shra; dw_OP_skip; 3; 0;
                                        dw_OP_plus_uconst; unskew + past_tag; dw_OP_deref; dw_OP_stack_value ]
  | Prim Nat32 -> Location.local slot [ dw_OP_dup; dw_OP_lit1; dw_OP_and; dw_OP_bra; 5; 0;
                                                 dw_OP_lit1; dw_OP_shr; dw_OP_skip; 3; 0;
                                                 dw_OP_plus_uconst; unskew + past_tag; dw_OP_deref; dw_OP_stack_value ]
  (* FIXME: for Int64|Nat64|Nat|Int the heap check is ignored for now *)
  | Prim Int64 -> Location.local slot [ dw_OP_lit1; dw_OP_shra; dw_OP_const4u; 0xFF; 0xFF; 0xFF; 0xFF; dw_OP_and; dw_OP_stack_value ]
  | Prim Nat64 -> Location.local slot [ dw_OP_lit1; dw_OP_shr; dw_OP_const4u; 0x7F; 0xFF; 0xFF; 0xFF; dw_OP_and; dw_OP_stack_value ]
  | Prim (Nat|Int) -> Location.local slot [ dw_OP_lit1; dw_OP_shra; dw_OP_stack_value ]

  | Tup _ -> Location.local slot []
  | Con (c, _) as ty ->
    begin match obvious_prim_of_con c ty with
    | Some p -> loc slot (Prim p)
    | _ -> Location.local slot [ dw_OP_stack_value ] (* FIXME: locate real type *)
    end
  | _ -> Location.local slot [ dw_OP_stack_value ] (* FIXME: objects, options *)


(* Initiate a (hierarchical) DWARF tag
   this might produce a list of DIEs
   and might need a subsequent closing DIE.
   See Note [emit a DW_TAG] *)
let tag_open : dw_TAG -> die list =
  let prim_type prim = fst (prim_type_ref prim) in
  let type_ ty = fst (type_ref ty) in
  let append_tag ds t ats = ds @ [unreferencable_tag t ats] in
  let open Type in
  function
  | Compile_unit (dir, file) ->
    let base_types = (* these are emitted for inspectionability, now *)
      List.concat_map prim_type
        [ Bool; Char; Text; Nat8; Int8; Nat16
        ; Int16; Nat32; Int32; Nat64; Int64] in
    let builtin_types =
      type_ Any @
      prim_type Nat @
      prim_type Int in
    [unreferencable_tag dw_TAG_compile_unit
       (dw_attrs
          [ Producer (Printf.sprintf "DFINITY Motoko compiler %s" Source_id.banner);
            Language dw_LANG_Motoko; Name file; Stmt_list 0;
            Comp_dir dir; Use_UTF8 true; Low_pc; Addr_base 8; Ranges ] @
        base_types @ builtin_types)]
  | Subprogram (name, [retty], pos) ->
    let ds, ref_ret = type_ref retty in
    append_tag ds Wasm_exts.Abbreviation.dw_TAG_subprogram_Ret
      (dw_attrs [Low_pc; High_pc; Name name; TypeRef ref_ret; Decl_file pos.Source.file; Decl_line pos.Source.line; Decl_column pos.Source.column; Prototyped true; External false])
  | Subprogram (name, _, pos) ->
    [unreferencable_tag dw_TAG_subprogram
       (dw_attrs [Low_pc; High_pc; Name name; Decl_file pos.Source.file; Decl_line pos.Source.line; Decl_column pos.Source.column; Prototyped true; External false])]
  | Formal_parameter (name, pos, ty, slot) ->
    let ds, reference = type_ref ty in
    append_tag ds dw_TAG_formal_parameter
      (dw_attrs [Name name; Decl_line pos.Source.line; Decl_column pos.Source.column; TypeRef reference; Location (loc slot ty)])
  | LexicalBlock pos ->
    [unreferencable_tag dw_TAG_lexical_block
       (dw_attrs [Decl_line pos.Source.line; Decl_column pos.Source.column])]
  | Variable (name, pos, ty, slot) ->
    let ds, reference = type_ref ty in
    append_tag ds dw_TAG_variable
      (dw_attrs [Name name; Decl_line pos.Source.line; Decl_column pos.Source.column; TypeRef reference; Location (loc slot ty)])
  | Type ty -> type_ ty
  | _ -> assert false
