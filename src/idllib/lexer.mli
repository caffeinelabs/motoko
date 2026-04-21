val token : Lexing.lexbuf -> Parser.token  (* raise ParseError *)
val region : Lexing.lexbuf -> Source.region
