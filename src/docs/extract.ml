open Mo_def
open Source

type doc = {
  xref : Xref.t;
  doc_comment : string option;
  declaration : declaration_doc;
}

and declaration_doc =
  | Function of function_doc
  | Value of value_doc
  | Type of type_doc
  | Class of class_doc
  | Object of object_doc
  | Unknown of string

and function_doc = {
  name : string;
  typ : Syntax.typ option;
  type_args : Syntax.typ_bind list;
  args : function_arg_doc list;
}

and function_arg_named = {
  name : string;
  typ : Syntax.typ option;
  doc : string option;
}

and function_arg_doc =
  | FANamed of function_arg_named
  | FAObject of function_arg_named list

and value_sort = Let | Var
and value_doc = { sort : value_sort; name : string; typ : Syntax.typ option }

and type_doc = {
  name : string;
  type_args : Syntax.typ_bind list;
  typ : doc_type;
}

and doc_type =
  | DTPlain of Syntax.typ
  (* One level unwrapping of an object type with documentation on its fields *)
  | DTObj of Syntax.typ * (Syntax.typ_field * string option) list

(* TODO We'll also want to unwrap variants here *)
and class_doc = {
  name : string;
  type_args : Syntax.typ_bind list;
  constructor : function_arg_doc list;
  fields : doc list;
  sort : Syntax.obj_sort;
}

and object_doc = { name : string; fields : doc list; sort : Syntax.obj_sort }

let un_prog prog =
  let comp_unit = Mo_def.CompUnit.comp_unit_of_prog true prog in
  let open Syntax in
  let { imports; body } = comp_unit.it in
  let imports =
    List.concat_map
      (fun i ->
        match i.it with
        | { it = VarP alias; _ }, path, _ -> [ (alias.it, path) ]
        | _ -> []) (* FIXME: explicit imports #3078 *)
      imports
  in
  match body.it with
  | ProgU _ -> Ok ([], []) (* treat all fields as private *)
  | MixinU (_, decs) ->
      Ok ([], decs) (* TODO figure out how to document mixin parameters *)
  | ModuleU (_, decs) -> Ok (imports, decs)
  | ActorU (_, _, _, decs) -> Ok (imports, decs)
  | ActorClassU (_, _, _, _, _, _, _, _, decs) ->
      let _, decs = CompUnit.decs_of_lib comp_unit in
      let decs =
        List.map
          (fun d ->
            { vis = Public None @@ no_region; dec = d; stab = None } @@ d.at)
          decs
      in
      Ok (imports, decs)

module PosTable = Trivia.PosHashtbl

type extracted = {
  module_comment : string option;
  lookup_type : Syntax.typ_path -> Xref.t option;
  docs : doc list;
}

module MakeExtract (Env : sig
  val all_decs : Syntax.dec_field list
  val imports : (string * string) list
  val find_trivia : Source.region -> Trivia.trivia_info
end) =
struct
  let namespace : Namespace.t =
    let import_ns = Namespace.from_imports Env.imports in
    let module_ns = Namespace.from_module Env.all_decs in
    Namespace.shadow import_ns module_ns

  let lookup_type : Syntax.typ_path -> Xref.t option =
    Namespace.lookup_type namespace

  let rec extract_args = function
    | { it = Syntax.VarP { it = name; at; _ }; _ } ->
        Some
          (FANamed
             {
               name;
               typ = None;
               doc = Trivia.doc_comment_of_trivia_info (Env.find_trivia at);
             })
    | { it = Syntax.AnnotP (p, ty); at; _ } ->
        Option.bind (extract_args p) (function
          | FANamed x ->
              Some
                (FANamed
                   {
                     x with
                     typ = Some ty;
                     doc =
                       Trivia.doc_comment_of_trivia_info (Env.find_trivia at);
                   })
          | FAObject _ -> None)
    | { it = Syntax.WildP; _ } -> None
    | { it = Syntax.ObjP fs; at; _ } ->
        let fields = List.filter_map extract_pat_field fs in
        Some (FAObject fields)
    | pat ->
        (* Wasm.Sexpr.print 80 (Arrange.pat pat); *)
        None

  and extract_pat_field pf =
    match pf.it with
    | Syntax.ValPF (id, { it = Syntax.AnnotP (_, typ); _ }) ->
        Some { name = id.it; typ = Some typ; doc = None }
    | _ -> None

  let extract_func_args = function
    | { it = Syntax.ParP arg; _ } -> Option.to_list (extract_args arg)
    | { it = Syntax.TupP args; _ } -> List.filter_map extract_args args
    | _ -> []

  let extract_typ_item (id_opt, typ) =
    FANamed
      {
        name = (match id_opt with Some id -> id.it | _ -> "_");
        typ = Some typ;
        doc = None;
      }

  let extract_ty_args = function
    | { it = Syntax.ParT arg; _ } ->
        [ FANamed { name = "_"; typ = Some arg; doc = None } ]
    | { it = Syntax.NamedT ({ it = name; _ }, arg); _ } ->
        [ FANamed { name; typ = Some arg; doc = None } ]
    | { it = Syntax.TupT args; _ } -> List.map extract_typ_item args
    | typ -> [ FANamed { name = "_"; typ = Some typ; doc = None } ]

  let is_func_ty ty =
    match ty.it with
    | Syntax.FuncT (_, ty_args, dom, cod) -> Some (ty_args, dom, cod)
    | _ -> None

  let extract_value_doc : value_sort -> Syntax.exp -> string -> declaration_doc
      =
   fun sort exp name ->
    match exp.it with
    | Syntax.FuncE (_, _, type_args, args, typ, _, _) ->
        let args_doc = extract_func_args args in
        Function { name; typ; type_args; args = args_doc }
    | Syntax.AnnotE (e, ty) when is_func_ty ty <> None -> (
        match is_func_ty ty with
        | Some (type_args, args, res) ->
            Function
              { name; typ = Some res; type_args; args = extract_ty_args args }
        | _ -> assert false)
    | Syntax.AnnotE (e, ty) -> Value { sort; name; typ = Some ty }
    | _ -> Value { sort; name; typ = None }

  let extract_obj_field_doc :
      Syntax.typ_field -> Syntax.typ_field * string option =
   fun ({ at; _ } as tf) ->
    (tf, Trivia.doc_comment_of_trivia_info (Env.find_trivia at))

  let rec extract_doc mk_xref = function
    | Source.
        {
          it = Syntax.LetD ({ it = Syntax.VarP { it = name; _ }; _ }, rhs, _);
          _;
        } -> (
        match rhs with
        | Source.{ it = Syntax.ObjBlockE (_, sort, _, fields); _ } ->
            let mk_field_xref xref = mk_xref (Xref.XClass (name, xref)) in
            Some
              ( mk_xref (Xref.XType name),
                Object
                  {
                    name;
                    fields =
                      List.filter_map (extract_dec_field mk_field_xref) fields;
                    sort;
                  } )
        | _ -> Some (mk_xref (Xref.XValue name), extract_value_doc Let rhs name)
        )
    | Source.{ it = Syntax.VarD ({ it = name; _ }, rhs); _ } -> (
        match rhs with
        | Source.{ it = Syntax.ObjBlockE (_, sort, _, fields); _ } ->
            let mk_field_xref xref = mk_xref (Xref.XClass (name, xref)) in
            Some
              ( mk_xref (Xref.XType name),
                Object
                  {
                    name;
                    fields =
                      List.filter_map (extract_dec_field mk_field_xref) fields;
                    sort;
                  } )
        | _ -> Some (mk_xref (Xref.XValue name), extract_value_doc Var rhs name)
        )
    | Source.{ it = Syntax.TypD (name, ty_args, typ); _ } ->
        let doc_typ =
          match typ.it with
          | Syntax.ObjT (_, fields) ->
              let doc_fields = List.map extract_obj_field_doc fields in
              (* TODO Only unwrap the ObjT if at least one field is documented *)
              DTObj (typ, doc_fields)
          | _ -> DTPlain typ
        in
        Some
          ( mk_xref (Xref.XType name.it),
            Type { name = name.it; type_args = ty_args; typ = doc_typ } )
    | Source.
        {
          it =
            Syntax.ClassD
              ( exp_opt,
                shared_pat,
                obj_sort,
                name,
                type_args,
                ctor,
                _,
                _,
                fields );
          _;
        } ->
        let mk_field_xref xref = mk_xref (Xref.XClass (name.it, xref)) in
        Some
          ( mk_xref (Xref.XType name.it),
            Class
              {
                name = name.it;
                type_args;
                constructor = extract_func_args ctor;
                fields =
                  List.filter_map (extract_dec_field mk_field_xref) fields;
                sort = obj_sort;
              } )
    | unknown ->
        print_endline "Failed to extract documentation for declaration:";
        Wasm.Sexpr.print 80 (Arrange.dec unknown);
        None

  and extract_dec_field mk_xref dec_field =
    if Syntax.is_public dec_field.it.Syntax.vis then
      extract_doc mk_xref dec_field.it.Syntax.dec
      |> Option.map (fun (xref, decl_doc) ->
             {
               xref;
               doc_comment =
                 Trivia.doc_comment_of_trivia_info
                   (Env.find_trivia dec_field.at);
               declaration = decl_doc;
             })
    else None
end

let extract_docs : Syntax.prog -> (extracted, string) result =
 fun prog ->
  let lookup_trivia (line, column) =
    PosTable.find_opt prog.note.Syntax.trivia Trivia.{ line; column }
  in
  let find_trivia (parser_pos : Source.region) : Trivia.trivia_info =
    lookup_trivia Source.(parser_pos.left.line, parser_pos.left.column)
    |> Option.get
  in
  let module_docs = find_trivia prog.at in
  (* Skip the module header *)
  match un_prog prog with
  | Ok (imports, decls) ->
      let module Ex = MakeExtract (struct
        let all_decs = decls
        let imports = imports
        let find_trivia = find_trivia
      end) in
      let docs = List.filter_map (Ex.extract_dec_field Fun.id) decls in
      Ok
        {
          module_comment = Trivia.doc_comment_of_trivia_info module_docs;
          lookup_type = Ex.lookup_type;
          docs;
        }
  | Error msg -> Error msg
