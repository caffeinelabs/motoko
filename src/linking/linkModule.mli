(* The arguments are:
   - the base module
   - the name of the library to link
   - the module containing that library
*)
exception LinkError of string
exception TooLargeDataSegments of string

(** Link a library into a base module. [finalize] and [keep_exports] both
    default to [true]. Set [finalize] to [false] for intermediate links.
    Set [keep_exports] to [false] for an independent library whose exports
    only resolve the imports for this link. Exports of the base module remain
    available for later links until [finalize]. *)
val link :
  ?finalize:bool ->
  ?keep_exports:bool ->
  Wasm_exts.CustomModule.extended_module ->
  string ->
  Wasm_exts.CustomModule.extended_module ->
  Wasm_exts.CustomModule.extended_module

(** Remove exports that exist only for static linking. Intermediate links must
    use [~finalize:false] until all libraries have been linked. *)
val finalize :
  Wasm_exts.CustomModule.extended_module ->
  Wasm_exts.CustomModule.extended_module

(** Check the compute plugin runtime ABI. This checks linking compatibility;
    plugins remain trusted native compiler extensions, not sandboxed code. *)
val validate_compute_plugin : Wasm_exts.CustomModule.extended_module -> unit
