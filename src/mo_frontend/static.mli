open Mo_def

val exp :  Diag.msg_store -> Syntax.exp -> unit
val module_fields : Diag.msg_store -> Syntax.dec_field list -> unit
val actor_fields : Diag.msg_store -> Syntax.dec_field list -> unit
val prog :  Syntax.prog -> unit Diag.result
