(* Shared state for passing migration IDs from typing to desugaring *)

let migration_ids_map : (Source.region, string list) Hashtbl.t = 
  Hashtbl.create 16

