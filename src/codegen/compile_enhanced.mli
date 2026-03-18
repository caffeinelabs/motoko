module Make (_ : Backend_intf.S) : sig
  open Ir_def

  val compile : Mo_config.Flags.compile_mode -> Wasm_exts.CustomModule.extended_module option -> Ir.prog -> Wasm_exts.CustomModule.extended_module
end
