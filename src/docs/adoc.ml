open Extract
open Printf
open Mo_def

type env = { lookup_type : Syntax.typ_path -> Xref.t option }

let surround : Buffer.t -> string -> (unit -> unit) -> unit =
 fun buf s inner ->
  Buffer.add_string buf s;
  inner ();
  Buffer.add_string buf s

let adoc_link : Xref.t -> string -> string =
 fun xref adoc_text ->
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
        Printf.sprintf "%s.adoc#%s" p (string_of_xref false xref)
  in
  let link = string_of_xref true xref in
  if link = "" then adoc_text else sprintf "xref:%s[%s]" link adoc_text

let adoc_render_path : env -> Syntax.typ_path -> string =
 fun env path ->
  match env.lookup_type path with
  | None -> Plain.string_of_path path
  | Some xref -> adoc_link xref (Plain.string_of_path path)

let render_fns env =
  Plain.
    {
      render_path = adoc_render_path env;
      render_open_bracket = "pass:[[]";
      render_close_bracket = "pass:[]]";
    }

let adoc_of_type : Buffer.t -> env -> Syntax.typ -> unit =
 fun buf env ty -> Plain.plain_of_typ buf (render_fns env) ty

let opt_typ : Buffer.t -> env -> Syntax.typ option -> unit =
 fun buf env ->
  Option.iter (fun ty ->
      bprintf buf " : ";
      adoc_of_type buf env ty)

let adoc_of_typ_bind : Buffer.t -> env -> Syntax.typ_bind -> unit =
 fun buf env -> Plain.plain_of_typ_bind buf (render_fns env)

let adoc_of_named_arg : Buffer.t -> env -> function_arg_named -> unit =
 fun buf env arg ->
  Buffer.add_string buf arg.name;
  opt_typ buf env arg.typ

let adoc_of_function_arg : Buffer.t -> env -> function_arg_doc -> unit =
 fun buf env -> function
  | FAObject fields ->
      bprintf buf "{ ";
      Plain.sep_by buf "; " (adoc_of_named_arg buf env) fields;
      bprintf buf " }"
  | FANamed arg ->
      Buffer.add_string buf arg.name;
      opt_typ buf env arg.typ

let adoc_header : Buffer.t -> int -> string -> unit =
 fun buf lvl s -> bprintf buf "%s %s\n\n" (String.make (lvl + 1) '=') s

let rec lvl_of_xref : Xref.t -> int = function
  | Xref.XType _ | Xref.XValue _ -> 1
  | Xref.XNested (_, xref) | Xref.XClass (_, xref) -> lvl_of_xref xref + 1
  | Xref.XFile (file, xref) -> 0
  | Xref.XPackage (pkg, xref) -> 0

let adoc_xref : Buffer.t -> Xref.t -> unit =
 fun buf xref ->
  let rec go = function
    | Xref.XType t -> bprintf buf "type.%s" t
    | Xref.XValue t -> bprintf buf "%s" t
    | Xref.XNested (x, xref) ->
        bprintf buf "%s." x;
        go xref
    | Xref.XClass (x, xref) ->
        bprintf buf "%s." x;
        go xref
    | Xref.XFile (file, xref) -> ()
    | Xref.XPackage (pkg, xref) -> ()
  in
  bprintf buf "[[";
  go xref;
  bprintf buf "]]\n"

let adoc_signature : Buffer.t -> (unit -> unit) -> unit =
 fun buf f ->
  bprintf buf "[source.no-repl,motoko,subs=+macros]\n----\n";
  f ();
  bprintf buf "\n----\n\n"

let adoc_of_doc_type : Buffer.t -> env -> Extract.doc_type -> unit =
 fun buf env dt ->
  match dt with
  | DTPlain t -> adoc_of_type buf env t
  | DTObj (t, fields) -> adoc_of_type buf env t

let rec adoc_of_declaration :
    Buffer.t -> env -> Xref.t -> (unit -> unit) -> declaration_doc -> unit =
 fun buf env xref doc_comment doc ->
  let header = adoc_header buf (lvl_of_xref xref) in
  let signature = adoc_signature buf in
  adoc_xref buf xref;
  match doc with
  | Function function_doc ->
      header function_doc.name;
      signature (fun _ ->
          bprintf buf "func %s" function_doc.name;
          Plain.sep_by' buf "<" ", " ">" (adoc_of_typ_bind buf env)
            function_doc.type_args;
          bprintf buf "(";
          Plain.sep_by buf ", " (adoc_of_function_arg buf env) function_doc.args;
          bprintf buf ")";
          opt_typ buf env function_doc.typ);
      doc_comment ()
  | Value value_doc ->
      header value_doc.name;
      signature (fun _ ->
          let sort = match value_doc.sort with Let -> "let" | Var -> "var" in
          bprintf buf "%s %s" sort value_doc.name;
          opt_typ buf env value_doc.typ);
      doc_comment ()
  | Type type_doc ->
      header type_doc.name;
      signature (fun _ ->
          bprintf buf "type %s" type_doc.name;
          Plain.sep_by' buf "<" ", " ">" (adoc_of_typ_bind buf env)
            type_doc.type_args;
          bprintf buf " = ";
          adoc_of_doc_type buf env type_doc.typ);
      doc_comment ()
  | Class class_doc ->
      header class_doc.name;
      signature (fun _ ->
          bprintf buf "class %s" class_doc.name;
          Plain.sep_by' buf "<" ", " ">" (adoc_of_typ_bind buf env)
            class_doc.type_args;
          bprintf buf "(";
          Plain.sep_by buf ", "
            (adoc_of_function_arg buf env)
            class_doc.constructor;
          bprintf buf ")");
      doc_comment ();
      bprintf buf "\n\n";
      List.iter (adoc_of_doc buf env) class_doc.fields
  | Object obj_doc ->
      header obj_doc.name;
      signature (fun _ -> bprintf buf "%s" obj_doc.name);
      doc_comment ();
      bprintf buf "\n\n";
      List.iter (adoc_of_doc buf env) obj_doc.fields
  | Unknown unknown ->
      signature (fun _ -> bprintf buf "Unknown %s" unknown);
      doc_comment ()

and adoc_of_doc : Buffer.t -> env -> doc -> unit =
 fun buf env { doc_comment; declaration; xref } ->
  let doc_comment () =
    Option.iter (Buffer.add_string buf) doc_comment;
    bprintf buf "\n\n"
  in
  adoc_of_declaration buf env xref doc_comment declaration

let render_docs : Common.render_input -> string =
 fun Common.
       {
         package_opt;
         module_comment;
         declarations;
         current_path;
         lookup_type;
         _;
       } ->
  let buf = Buffer.create 1024 in
  bprintf buf "[[module.%s]]\n= %s%s\n\n" current_path
    (match package_opt with Some s -> s ^ "/" | None -> "")
    current_path;
  Option.iter (bprintf buf "%s\n\n") module_comment;
  let env = { lookup_type } in
  List.iter (adoc_of_doc buf env) declarations;
  Buffer.contents buf
