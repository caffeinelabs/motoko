open Mo_def

val exp : ?allow_system:bool -> Diag.msg_store -> Syntax.exp -> unit
val dec_fields : ?allow_system:bool -> Diag.msg_store -> Syntax.dec_field list -> unit
val prog : ?allow_system:bool -> Syntax.prog -> unit Diag.result
