open Mo_def
module StringMap : Map.S with type key = string

type t = {
  types : Xref.t StringMap.t;
  values : (Xref.t * t option) StringMap.t;
}

val from_imports : (string * string) list -> t

(** Definitions in the second namespace shadow the first one *)
val shadow : t -> t -> t

(** Extracts the shape of a namespace for a given module from its syntactic
    representation. *)
val from_module : Syntax.dec_field list -> t

val lookup_type : t -> Syntax.typ_path -> Xref.t option
val to_string : t -> string
