(* Recognize only bodies whose complete semantics are a primitive ordering.
   [resolve_prim] looks through already known constant function values; it
   must not infer primitive semantics from a function or module name. *)
val recognize :
  resolve_prim:(Ir_def.Ir.exp -> Ir_def.Ir.prim option) ->
  Ir_def.Ir.exp -> Mo_types.Type.prim option

(* Internal classifier tags, independent of plugin ABI and heap layout. *)
val tag : Mo_types.Type.prim -> int64
