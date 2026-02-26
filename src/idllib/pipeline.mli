val check_file : string -> (Syntax.prog * Typing.scope * Syntax.typ option) Diag.result
val check_prog : Syntax.prog -> Typing.scope Diag.result
val compile_js_string : string -> Buffer.t Diag.result
val compile_js_file : string -> Buffer.t Diag.result

val parse_string : ?left:Source.pos -> string -> (Syntax.prog * string) Diag.result
val parse_test_file : string -> Syntax.tests Diag.result
val parse_values : string -> Syntax.args Diag.result
