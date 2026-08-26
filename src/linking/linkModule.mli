(* The arguments are:
   - the base module
   - the name of the library to link
   - the module containing that library
*)
exception LinkError of string
exception TooLargeDataSegments of string

val link : Wasm_exts.CustomModule.extended_module -> string -> Wasm_exts.CustomModule.extended_module -> Wasm_exts.CustomModule.extended_module

(* spike: merge a moc-produced frozen migration object (see
   design/MigrationObjects.md, Phase 1) into an unlinked moc main module *)
val link_mco : Wasm_exts.CustomModule.extended_module -> Wasm_exts.CustomModule.extended_module -> Wasm_exts.CustomModule.extended_module
