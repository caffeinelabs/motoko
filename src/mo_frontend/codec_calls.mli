open Mo_def

(* Warn (M0281) about typed calls to actor methods that have an `encoder` or
   a `decoder`, see `codec_calls.ml` *)
val check_prog : Syntax.prog -> unit Diag.result
val check_lib : Syntax.lib -> unit Diag.result
