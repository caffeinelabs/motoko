(* .moi (Motoko Object Interface) file cache for incremental compilation.
   Serializes Scope.t to disk so that unchanged library dependencies can
   be loaded without re-parsing and re-type-checking their sources. *)

type fingerprint = string  (* Digest.t — 16-byte MD5 *)

type header = {
  source_hash : fingerprint;
  scope_fingerprint : fingerprint;
  deps : (string * fingerprint) list;
}

(* Compute MD5 of a file's contents *)
val hash_file : string -> fingerprint

(* Merkle fingerprint: hash(source_hash || sorted dep fingerprints) *)
val compute_fingerprint :
  source_hash:fingerprint -> deps:(string * fingerprint) list -> fingerprint

(* Derive .moi path from cache dir and resolved import name *)
val moi_path : cache_dir:string -> import_key:string -> string

(* Try to load a cached scope.
   Returns None if the file is missing, corrupt, or fails validation.
   [dep_fingerprints] maps dep resolved-names to their current fingerprints. *)
val load :
  cache_dir:string ->
  source_path:string ->
  source_hash:fingerprint ->
  dep_fingerprints:(string -> fingerprint option) ->
  (header * Mo_frontend.Scope.t) option

(* Write a scope to the cache. Creates the cache directory if needed. *)
val save :
  cache_dir:string ->
  source_path:string ->
  header:header ->
  scope:Mo_frontend.Scope.t ->
  unit
