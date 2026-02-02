open Mo_def

type render_input = {
  package_opt : string option;
  all_modules : string list;  (** Needed so we can generate a navigation *)
  current_path : string;  (** The path for the current module *)
  module_comment : string option;
      (** The top-level module comment for the current module *)
  lookup_type : Syntax.typ_path -> Xref.t option;
      (** How to look up references to types in the current module *)
  declarations : Extract.doc list;  (** The list of declarations to process *)
}

let is_scope_bind : Syntax.typ_bind -> bool =
 fun typ_bind ->
  match typ_bind.Source.it.Syntax.sort.Source.it with
  | Mo_types.Type.Scope -> true
  | _ -> false
