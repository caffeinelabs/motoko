val wasm_release : string Lazy.t
val wasm_debug : string Lazy.t
(* The runtimes with native tables; None in a compiler built without them, such as moc.js *)
val wasm_release_table : string Lazy.t option
val wasm_debug_table : string Lazy.t option
