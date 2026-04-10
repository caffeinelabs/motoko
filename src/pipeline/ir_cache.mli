type cache_data = {
  decs : Ir_def.Ir.dec list;
  id_stamps : (string * int) list;
}

val write : string -> dep_hash:string -> cache_data -> unit
val read : string -> dep_hash:string -> cache_data option
val moic_path : cache_dir:string -> dep_hash:string -> string
