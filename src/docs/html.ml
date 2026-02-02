open Extract
open Mo_def
open Mo_types
open Cow.Html
open Common

type env = { lookup_type : Syntax.typ_path -> Xref.t option }

let rec join_with : t -> t list -> t =
 fun sep -> function
  | [] -> empty
  | [ x ] -> x
  | x :: xs -> x ++ sep ++ join_with sep xs

let space : t = string "\u{00A0}"
let cls_span : string -> string -> t = fun cls s -> span ~cls (string s)
let fn_name : string -> t = cls_span "fnname"
let class_name : string -> t = cls_span "classname"
let object_name : string -> t = cls_span "objectname"
let keyword : string -> t = cls_span "keyword"
let parameter : string -> t = cls_span "parameter"
let html_type : string -> t = cls_span "type"

let rec string_of_path' : Syntax.path' -> string = function
  | Syntax.IdH id -> id.Source.it
  | Syntax.DotH (path, id) ->
      string_of_path' path.Source.it ^ "." ^ id.Source.it

let string_of_path : Syntax.typ_path -> string =
 fun path -> string_of_path' path.Source.it

let rec id_of_xref : Xref.t -> string = function
  | Xref.XClass (x, xref) -> Printf.sprintf "%s.%s" x (id_of_xref xref)
  | Xref.XNested (x, xref) -> Printf.sprintf "%s.%s" x (id_of_xref xref)
  | Xref.XValue x -> x
  | Xref.XType x -> "type." ^ x
  | Xref.XPackage (_, Some xref) -> id_of_xref xref
  | Xref.XFile (p, Some xref) -> id_of_xref xref
  (* Shouldn't happen, a File/Package gets a Url not an anchor *)
  | Xref.XPackage (_, None) | Xref.XFile (_, None) -> assert false

let link_of_xref : Xref.t -> t -> t =
 fun xref html ->
  let prepend_hash is_top s = if is_top then "#" ^ s else s in
  let rec string_of_xref is_top = function
    | Xref.XClass (x, xref) ->
        prepend_hash is_top
          (Printf.sprintf "%s.%s" x (string_of_xref false xref))
    | Xref.XNested (x, xref) ->
        prepend_hash is_top
          (Printf.sprintf "%s.%s" x (string_of_xref false xref))
    | Xref.XValue x -> prepend_hash is_top x
    | Xref.XType x -> prepend_hash is_top ("type." ^ x)
    | Xref.XPackage (_, _) -> ""
    | Xref.XFile (p, None) -> p
    | Xref.XFile (p, Some xref) ->
        Printf.sprintf "%s.html#%s" p (string_of_xref false xref)
  in
  let link = string_of_xref true xref in
  if link = "" then html else a ~href:(Uri.of_string link) html

let html_of_path : env -> Syntax.typ_path -> t =
 fun env path ->
  match env.lookup_type path with
  | Some xref -> link_of_xref xref (html_type (string_of_path path))
  | None -> html_type (string_of_path path)

let html_of_comment : string -> t = function
  | "" -> empty
  | s -> Cow.Markdown.of_string s

let html_of_mut : Syntax.mut -> t =
 fun mut ->
  match mut.Source.it with
  | Syntax.Var -> keyword "var "
  | Syntax.Const -> string ""

let html_of_func_sort : Syntax.func_sort -> t =
 fun sort ->
  Mo_types.Type.(
    match sort.Source.it with
    | Local -> empty
    | Shared Composite -> keyword "shared composite query "
    | Shared Query -> keyword "shared query "
    | Shared Write -> keyword "shared ")

let html_of_obj_sort : 'note Syntax.sort -> t =
 fun sort ->
  Mo_types.Type.(
    match sort.Source.it with
    | Object -> empty
    | Actor -> keyword "actor "
    | Module -> keyword "module "
    | Mixin -> keyword "mixin "
    | Memory -> keyword "memory ")

let rec html_of_type : env -> Syntax.typ -> t =
 fun env typ ->
  match typ.Source.it with
  | Syntax.PathT (path, typs) -> (
      html_of_path env path
      ++
      match typs with
      | [] -> empty
      | xs ->
          string "<"
          ++ join_with (string ", ") (List.map (html_of_type env) xs)
          ++ string ">")
  | Syntax.PrimT typ -> html_type typ
  | Syntax.ParT typ -> string "(" ++ html_of_type env typ ++ string ")"
  | Syntax.NamedT (id, t) ->
      string "(" ++ html_of_typ_item env (Some id, t) ++ string ")"
  | Syntax.OptT typ -> string "?" ++ html_of_type env typ
  | Syntax.WeakT typ -> string "weak" ++ html_of_type env typ
  | Syntax.TupT typ_list ->
      string "("
      ++ join_with (string ", ") (List.map (html_of_typ_item env) typ_list)
      ++ string ")"
  | Syntax.VariantT typ_tags ->
      string "{"
      ++ join_with (string "; ") (List.map (html_of_typ_tag env) typ_tags)
      ++ string "}"
  | Syntax.FuncT (func_sort, typ_binders, arg, res) ->
      let ty_args = html_of_typ_binders env typ_binders in
      let ty_arg = html_of_type env arg in
      html_of_func_sort func_sort
      ++ ty_args
      ++ ty_arg
      ++ string " -> "
      ++ html_of_type env res
  | Syntax.ArrayT (mut, ty) ->
      string "[" ++ html_of_mut mut ++ html_of_type env ty ++ string "]"
  | Syntax.AsyncT (Type.Fut, _scope, typ) ->
      keyword "async " ++ html_of_type env typ
  | Syntax.AsyncT (Type.Cmp, _scope, typ) ->
      keyword "async* " ++ html_of_type env typ
  | Syntax.AndT (typ1, typ2) ->
      html_of_type env typ1 ++ string " and " ++ html_of_type env typ2
  | Syntax.OrT (typ1, typ2) ->
      html_of_type env typ1 ++ string " or " ++ html_of_type env typ2
  | Syntax.ObjT (obj_sort, fields) ->
      html_of_obj_sort obj_sort
      ++ string "{ "
      ++ join_with (string "; ") (List.map (html_of_typ_field env) fields)
      ++ string " }"

and html_of_typ_tag : env -> Syntax.typ_tag -> t =
 fun env typ_tag ->
  string (Printf.sprintf "#%s" typ_tag.Source.it.Syntax.tag.Source.it)
  ++
  match typ_tag.Source.it.Syntax.typ.Source.it with
  | Syntax.TupT [] -> nil
  | _ -> string " : " ++ html_of_type env typ_tag.Source.it.Syntax.typ

and html_of_typ_bind : env -> Syntax.typ_bind -> t =
 fun env typ_bind ->
  let bound = typ_bind.Source.it.Syntax.bound in
  let bound_html =
    if Syntax.is_any bound then empty
    else string " <: " ++ html_of_type env bound
  in
  html_type typ_bind.Source.it.Syntax.var.Source.it ++ bound_html

and html_of_typ_binders : env -> Syntax.typ_bind list -> t =
 fun env typ_binders ->
  match List.filter (fun b -> not (is_scope_bind b)) typ_binders with
  | [] -> []
  | xs ->
      string "<"
      ++ join_with (string ", ") (List.map (html_of_typ_bind env) xs)
      ++ string ">"

and html_of_typ_field : env -> Syntax.typ_field -> t =
 fun env field ->
  (* TODO mut might be wrong here *)
  match field.Source.it with
  | Syntax.ValF (id, typ, mut) ->
      html_of_mut mut ++ string (id.Source.it ^ " : ") ++ html_of_type env typ
  | Syntax.TypF (id, tbs, typ) ->
      let ty_args = html_of_typ_binders env tbs in
      string "type "
      ++ string id.Source.it
      ++ ty_args
      ++ string " = "
      ++ html_of_type env typ

and html_of_typ_item : env -> Syntax.typ_item -> t =
 fun env (oid, t) ->
  Option.fold ~none:empty
    ~some:(fun id -> parameter id.Source.it ++ string " : ")
    oid
  ++ html_of_type env t

let html_of_type_doc : env -> Extract.type_doc -> Xref.t -> t =
 fun env type_doc xref ->
  let ty_args = html_of_typ_binders env type_doc.type_args in
  let id = id_of_xref xref in
  match type_doc.typ with
  | DTPlain ty ->
      h4 ~cls:"type-declaration" ~id
        (keyword "type "
        ++ html_type type_doc.name
        ++ ty_args
        ++ string " = "
        ++ html_of_type env ty)
  | DTObj (ty, fields) ->
      h4 ~cls:"type-declaration" ~id
        (keyword "type "
        ++ html_type type_doc.name
        ++ ty_args
        ++ string " = "
        ++ html_of_type env ty)

let html_of_named_arg : env -> Extract.function_arg_named -> t =
 fun env arg ->
  parameter arg.name
  ++ Option.fold ~none:empty
       ~some:(fun arg -> string " : " ++ html_of_type env arg)
       arg.typ

let html_of_arg : env -> Extract.function_arg_doc -> t =
 fun env -> function
  | FAObject fields ->
      string "{ "
      ++ join_with (string "; ") (List.map (html_of_named_arg env) fields)
      ++ string " }"
  | FANamed arg -> html_of_named_arg env arg

let rec html_of_declaration : env -> Xref.t -> Extract.declaration_doc -> t =
 fun env xref dec ->
  let id = id_of_xref xref in
  match dec with
  | Function function_doc ->
      let is_multiline = List.length function_doc.args > 2 in
      let br' = if is_multiline then br else empty in
      let br_indent = if is_multiline then br ++ space ++ space else empty in
      let ty_args = html_of_typ_binders env function_doc.type_args in
      (* TODO: Figure out a layout to show documentation for individual
       *  arguments *)
      let args =
        join_with
          (string ", " ++ br_indent)
          (List.map (html_of_arg env) function_doc.args)
      in
      let return_typ =
        Option.fold ~none:[]
          ~some:(fun typ -> string " : " ++ html_of_type env typ)
          function_doc.typ
      in
      h4 ~cls:"function" ~id
        (code
           (keyword "public func "
           ++ fn_name function_doc.name
           ++ ty_args
           ++ string "("
           ++ br_indent
           ++ args
           ++ br'
           ++ string ")"
           ++ return_typ))
  | Class class_doc ->
      let is_multiline = List.length class_doc.constructor > 2 in
      let br' = if is_multiline then br else empty in
      let br_indent = if is_multiline then br ++ space ++ space else empty in
      let ty_args = html_of_typ_binders env class_doc.type_args in
      let args =
        join_with
          (string ", " ++ br_indent)
          (List.map (html_of_arg env) class_doc.constructor)
      in
      h4 ~cls:"class-declaration" ~id
        (html_of_obj_sort class_doc.sort
        ++ keyword "class "
        ++ class_name class_doc.name
        ++ ty_args
        ++ string "("
        ++ br_indent
        ++ args
        ++ br'
        ++ string ")")
      ++ list (List.map (html_of_doc env) class_doc.fields)
  | Object obj_doc ->
      h4 ~cls:"object-declaration" ~id
        (html_of_obj_sort obj_doc.sort ++ object_name obj_doc.name)
      ++ list (List.map (html_of_doc env) obj_doc.fields)
  | Type type_doc -> html_of_type_doc env type_doc xref
  | Value value_doc ->
      h4 ~cls:"value-declaration" ~id
        (code
           (keyword "public "
           ++ keyword
                (match value_doc.sort with Let -> "let " | Var -> "var ")
           ++ fn_name value_doc.name
           ++ string " : "
           ++ Option.fold ~none:empty ~some:(html_of_type env) value_doc.typ))
  | Unknown s -> code (string "Unknown: " ++ string s)

and html_of_doc : env -> Extract.doc -> t =
 fun env { doc_comment; declaration; xref } ->
  div ~cls:"declaration"
    (html_of_declaration env xref declaration
    ++ p (html_of_comment (doc_comment |> Option.value ~default:"")))

let html_of_docs : render_input -> Cow.Html.t =
 fun {
       package_opt;
       all_modules;
       module_comment;
       declarations;
       lookup_type;
       current_path;
     } ->
  let env = { lookup_type } in
  let path_to_root =
    String.split_on_char '/' current_path
    |> List.tl
    |> List.map (fun _ -> "../")
    |> String.concat ""
  in
  let header =
    head
      ~attrs:[ ("title", "Doc") ]
      (meta ~charset:"UTF-8" []
      ++ link ~rel:"stylesheet" (Uri.of_string (path_to_root ^ "styles.css")))
  in
  let nav_of_doc doc =
    match doc.Extract.declaration with
    | Extract.Function func ->
        li (a ~href:(Uri.of_string ("#" ^ func.name)) (string func.name))
    | Extract.Type typ ->
        li (a ~href:(Uri.of_string ("#type." ^ typ.name)) (string typ.name))
    | Extract.Class cls ->
        li (a ~href:(Uri.of_string ("#type." ^ cls.name)) (string cls.name))
    | Extract.Object obj ->
        li (a ~href:(Uri.of_string ("#type." ^ obj.name)) (string obj.name))
    | Extract.Value val' ->
        li (a ~href:(Uri.of_string ("#" ^ val'.name)) (string val'.name))
    | Extract.Unknown typ -> empty
  in
  let navigation =
    nav ~cls:"sidebar"
      (h3 (string "Modules")
      ++ ul
           (List.map
              (fun path ->
                li
                  (a
                     ~href:(Uri.of_string (path_to_root ^ path ^ ".html"))
                     (string path)))
              all_modules)
      ++ h3 (string "Declarations")
      ++ ul (List.map nav_of_doc declarations))
  in
  let bdy =
    body
      (navigation
      ++ div ~cls:"documentation"
           (h1
              (string
                 (Printf.sprintf "%s%s"
                    (match package_opt with Some s -> s ^ "/" | None -> "")
                    current_path))
           ++ html_of_comment (Option.value ~default:"" module_comment)
           ++ list (List.map (html_of_doc env) declarations)))
  in
  html (header ++ bdy)

let render_docs : render_input -> string =
 fun input -> Format.asprintf "%s" (Cow.Html.to_string (html_of_docs input))

let make_index : string option -> render_input list -> string =
 fun package_opt inputs ->
  let header =
    head
      ~attrs:[ ("title", "Motoko docs") ]
      (meta ~charset:"UTF-8" []
      ++ link ~rel:"stylesheet" (Uri.of_string "styles.css"))
  in
  let make_link input =
    a ~cls:"index-item-link"
      ~href:(Uri.of_string (input.current_path ^ ".html"))
      (string input.current_path)
    ++ div ~cls:"index-item-comment"
         (html_of_comment (Option.value ~default:"" input.module_comment))
  in
  let bdy =
    div ~cls:"index-container"
      (h1 ~cls:"index-header"
         (string
            (match package_opt with
            | None -> "Motoko package"
            | Some s -> "Motoko " ^ s ^ " package"))
      ++ ul ~cls:"index-listing" ~licls:"index-item" (List.map make_link inputs)
      )
  in
  let index = html (header ++ bdy) in
  Format.asprintf "%s" (Cow.Html.to_string index)
