open Mo_def

val exp : Diag.msg_store -> Syntax.exp -> unit
val dec_fields : Diag.msg_store -> Syntax.dec_field list -> unit
val prog : Syntax.prog -> unit Diag.result
