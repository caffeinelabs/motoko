open Extract
open Mo_def
open Mo_types
open Source
open Printf
open Common

type level = int

type render_functions = {
  render_path : Syntax.typ_path -> string;
  render_open_bracket : string;
  render_close_bracket : string;
}

let sep_by : Buffer.t -> string -> ('a -> unit) -> 'a list -> unit =
 fun buf sep f -> function
  | [] -> ()
  | x :: xs ->
      f x;
      List.iter
        (fun x ->
          Buffer.add_string buf sep;
          f x)
        xs

let sep_by' :
    Buffer.t -> string -> string -> string -> ('a -> unit) -> 'a list -> unit =
 fun buf open_ sep close f -> function
  | [] -> ()
  | x :: xs ->
      Buffer.add_string buf open_;
      f x;
      List.iter
        (fun x ->
          Buffer.add_string buf sep;
          f x)
        xs;
      Buffer.add_string buf close

(** Adds a title at `level` *)
let title : Buffer.t -> level -> string -> unit =
 fun buf level txt -> bprintf buf "\n%s %s" (String.make level '#') txt

let rec plain_of_path : Buffer.t -> Syntax.path' -> unit =
 fun buf -> function
  | Syntax.IdH id -> Buffer.add_string buf id.it
  | Syntax.DotH (path, id) ->
      plain_of_path buf path.it;
      bprintf buf ".%s" id.it

let string_of_path : Syntax.typ_path -> string =
 fun path ->
  let buf = Buffer.create 8 in
  plain_of_path buf path.it;
  Buffer.contents buf

let plain_render_functions : render_functions =
  {
    render_path = string_of_path;
    render_open_bracket = "[";
    render_close_bracket = "]";
  }

let plain_of_mut : Buffer.t -> Syntax.mut -> unit =
 fun buf mut ->
  match mut.it with
  | Syntax.Var -> Buffer.add_string buf "var "
  | Syntax.Const -> ()

let plain_of_func_sort : Buffer.t -> Syntax.func_sort -> unit =
 fun buf sort ->
  Mo_types.Type.(
    match sort.it with
    | Local -> ()
    | Shared Composite -> bprintf buf "shared composite query "
    | Shared Query -> bprintf buf "shared query "
    | Shared Write -> bprintf buf "shared ")

let plain_of_obj_sort_title : Buffer.t -> 'note Syntax.sort -> unit =
 fun buf sort ->
  Buffer.add_string buf
    Mo_types.Type.(
      match sort.it with
      | Object -> "Object "
      | Actor -> "Actor "
      | Mixin -> "Mixin "
      | Module -> "Module "
      | Memory -> "Memory ")

let plain_of_obj_sort : Buffer.t -> 'note Syntax.sort -> unit =
 fun buf sort ->
  Buffer.add_string buf
    Mo_types.Type.(
      match sort.it with
      | Object -> ""
      | Actor -> "actor "
      | Module -> "module "
      | Mixin -> "mixin "
      | Memory -> "memory ")

let rec plain_of_typ : Buffer.t -> render_functions -> Syntax.typ -> unit =
 fun buf rf typ ->
  match typ.Source.it with
  | Syntax.PathT (path, typs) ->
      bprintf buf "%s" (rf.render_path path);
      sep_by' buf "<" ", " ">" (plain_of_typ buf rf) typs
  | Syntax.PrimT typ -> Buffer.add_string buf typ
  | Syntax.ObjT (obj_sort, fields) ->
      plain_of_obj_sort buf obj_sort;
      bprintf buf "{ ";
      sep_by buf "; " (plain_of_typ_field buf rf) fields;
      bprintf buf " }"
  | Syntax.ArrayT (mut, ty) ->
      bprintf buf "%s" rf.render_open_bracket;
      plain_of_mut buf mut;
      plain_of_typ buf rf ty;
      bprintf buf "%s" rf.render_close_bracket
  | Syntax.OptT typ ->
      bprintf buf "?";
      plain_of_typ buf rf typ
  | Syntax.WeakT typ ->
      bprintf buf "weak";
      plain_of_typ buf rf typ
  | Syntax.VariantT typ_tags ->
      bprintf buf "{";
      sep_by buf "; " (plain_of_typ_tag buf rf) typ_tags;
      bprintf buf "}"
  | Syntax.TupT typ_list ->
      bprintf buf "(";
      sep_by buf ", " (plain_of_typ_item buf rf) typ_list;
      bprintf buf ")"
  | Syntax.AsyncT (Type.Fut, _scope, typ) ->
      bprintf buf "async ";
      plain_of_typ buf rf typ
  | Syntax.AsyncT (Type.Cmp, _scope, typ) ->
      bprintf buf "async* ";
      plain_of_typ buf rf typ
  | Syntax.AndT (typ1, typ2) ->
      plain_of_typ buf rf typ1;
      bprintf buf " and ";
      plain_of_typ buf rf typ2
  | Syntax.OrT (typ1, typ2) ->
      plain_of_typ buf rf typ1;
      bprintf buf " or ";
      plain_of_typ buf rf typ2
  | Syntax.ParT typ ->
      bprintf buf "(";
      plain_of_typ buf rf typ;
      bprintf buf ")"
  | Syntax.NamedT (id, typ) ->
      bprintf buf "(";
      plain_of_typ_item buf rf (Some id, typ);
      bprintf buf ")"
  | Syntax.FuncT (func_sort, typ_binders, arg, res) ->
      plain_of_func_sort buf func_sort;
      plain_of_typ_binders buf rf typ_binders;
      plain_of_typ buf rf arg;
      bprintf buf " -> ";
      plain_of_typ buf rf res

and plain_of_typ_tag : Buffer.t -> render_functions -> Syntax.typ_tag -> unit =
 fun buf rf typ_tag ->
  bprintf buf "#%s" typ_tag.it.Syntax.tag.it;
  match typ_tag.it.Syntax.typ.it with
  | Syntax.TupT [] -> ()
  | _ ->
      bprintf buf " : ";
      plain_of_typ buf rf typ_tag.it.Syntax.typ

and plain_of_typ_bind : Buffer.t -> render_functions -> Syntax.typ_bind -> unit
    =
 fun buf rf typ_bind ->
  let bound = typ_bind.it.Syntax.bound in
  Buffer.add_string buf typ_bind.it.Syntax.var.it;
  if not (Syntax.is_any bound) then (
    bprintf buf " <: ";
    plain_of_typ buf rf typ_bind.it.Syntax.bound)

and plain_of_typ_binders :
    Buffer.t -> render_functions -> Syntax.typ_bind list -> unit =
 fun buf rf typ_binders ->
  let typ_binders =
    List.filter (fun b -> not (Common.is_scope_bind b)) typ_binders
  in
  sep_by' buf "<" ", " ">" (plain_of_typ_bind buf rf) typ_binders

and plain_of_typ_field :
    Buffer.t -> render_functions -> Syntax.typ_field -> unit =
 fun buf rf field ->
  match field.Source.it with
  | Syntax.ValF (id, typ, mut) ->
      plain_of_mut buf mut;
      bprintf buf "%s : " id.it;
      plain_of_typ buf rf typ
  | Syntax.TypF (id, tbs, typ) ->
      bprintf buf "type %s" id.it;
      plain_of_typ_binders buf rf tbs;
      bprintf buf " = ";
      plain_of_typ buf rf typ

and plain_of_typ_item : Buffer.t -> render_functions -> Syntax.typ_item -> unit
    =
 fun buf rf (oid, t) ->
  Option.iter (fun id -> bprintf buf "%s : " id.it) oid;
  plain_of_typ buf rf t

let opt_typ : Buffer.t -> Syntax.typ option -> unit =
 fun buf ->
  Option.iter (fun ty ->
      bprintf buf " : ";
      plain_of_typ buf plain_render_functions ty)

let plain_of_doc_typ : Buffer.t -> doc_type -> unit =
 fun buf -> function
  | DTPlain ty -> plain_of_typ buf plain_render_functions ty
  | DTObj (ty, doc_fields) -> plain_of_typ buf plain_render_functions ty

let named_arg : Buffer.t -> function_arg_named -> unit =
 fun buf arg ->
  Buffer.add_string buf arg.name;
  opt_typ buf arg.typ

let function_arg : Buffer.t -> function_arg_doc -> unit =
 fun buf -> function
  | FAObject fields ->
      bprintf buf "{ ";
      sep_by buf "; " (named_arg buf) fields;
      bprintf buf " }"
  | FANamed arg -> named_arg buf arg

let begin_block buf = bprintf buf "\n``` motoko no-repl\n"
let end_block buf = bprintf buf "\n```\n\n"

let rec declaration_header :
    Buffer.t -> level -> (unit -> unit) -> declaration_doc -> unit =
 fun buf lvl doc_comment -> function
  | Function function_doc ->
      title buf lvl (Printf.sprintf "Function `%s`" function_doc.name);
      begin_block buf;
      bprintf buf "func %s" function_doc.name;
      plain_of_typ_binders buf plain_render_functions function_doc.type_args;
      bprintf buf "(";
      sep_by buf ", " (function_arg buf) function_doc.args;
      bprintf buf ")";
      opt_typ buf function_doc.typ;
      end_block buf;
      doc_comment ()
  | Value value_doc ->
      title buf lvl (Printf.sprintf "Value `%s`" value_doc.name);
      begin_block buf;
      let sort = match value_doc.sort with Let -> "let" | Var -> "var" in
      bprintf buf "%s %s" sort value_doc.name;
      opt_typ buf value_doc.typ;
      end_block buf;
      doc_comment ()
  | Type type_doc ->
      title buf lvl (Printf.sprintf "Type `%s`" type_doc.name);
      begin_block buf;
      bprintf buf "type %s" type_doc.name;
      plain_of_typ_binders buf plain_render_functions type_doc.type_args;
      bprintf buf " = ";
      plain_of_doc_typ buf type_doc.typ;
      end_block buf;
      doc_comment ()
  | Class class_doc ->
      title buf lvl "";
      (match class_doc.sort.it with
      | Type.Object -> ()
      | _ -> plain_of_obj_sort_title buf class_doc.sort);
      bprintf buf "Class `%s" class_doc.name;
      plain_of_typ_binders buf plain_render_functions class_doc.type_args;
      bprintf buf "`\n";
      begin_block buf;
      bprintf buf "class %s" class_doc.name;
      plain_of_typ_binders buf plain_render_functions class_doc.type_args;
      bprintf buf "(";
      sep_by buf ", " (function_arg buf) class_doc.constructor;
      bprintf buf ")";
      end_block buf;
      doc_comment ();
      sep_by buf "\n" (plain_of_doc buf (lvl + 1)) class_doc.fields
  | Object obj_doc ->
      title buf lvl "";
      plain_of_obj_sort_title buf obj_doc.sort;
      bprintf buf "`%s`\n" obj_doc.name;
      begin_block buf;
      plain_of_obj_sort buf obj_doc.sort;
      bprintf buf "%s" obj_doc.name;
      end_block buf;
      doc_comment ();
      sep_by buf "\n" (plain_of_doc buf (lvl + 1)) obj_doc.fields
  | Unknown u ->
      title buf lvl (Printf.sprintf "Unknown %s" u);
      doc_comment ()

and plain_of_doc : Buffer.t -> level -> doc -> unit =
 fun buf lvl { doc_comment; declaration; _ } ->
  let doc_comment () = Option.iter (bprintf buf "%s\n") doc_comment in
  declaration_header buf lvl doc_comment declaration

let render_docs : Common.render_input -> string =
 fun Common.{ package_opt; module_comment; declarations; current_path; _ } ->
  let buf = Buffer.create 1024 in
  bprintf buf "# %s%s\n"
    (match package_opt with Some s -> s ^ "/" | None -> "")
    current_path;
  Option.iter (bprintf buf "%s\n") module_comment;
  List.iter (plain_of_doc buf 2) declarations;
  Buffer.contents buf

let make_index : string option -> Common.render_input list -> string =
 fun package_opt inputs ->
  let buf = Buffer.create 1024 in
  bprintf buf "# Motoko %spackage\n\n"
    (match package_opt with None -> "" | Some s -> s ^ " ");
  List.iter
    (fun (input : Common.render_input) ->
      bprintf buf "* [%s](%s) %s\n" input.current_path
        (input.current_path ^ ".md")
        (match input.module_comment with
        | None -> ""
        | Some s -> (
            match String.split_on_char '\n' s with [] -> "" | l :: _ -> l)))
    inputs;
  Buffer.contents buf
