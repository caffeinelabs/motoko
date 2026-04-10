{
open Trivia
open Source_token
open Lexer_lib
open Source
module Utf8 = Lib.Utf8

let region lexbuf =
  let left = convert_pos (Lexing.lexeme_start_p lexbuf) in
  let right = convert_pos (Lexing.lexeme_end_p lexbuf) in
  {left; right}

let error lexbuf msg = raise (Error (region lexbuf, msg))
let error_nest start lexbuf msg =
  lexbuf.Lexing.lex_start_p <- start;
  error lexbuf msg

let utf8 s i =
  let len = if s.[!i] < '\xe0' then 1 else if s.[!i] < '\xf0' then 2 else 3 in
  i := !i + len;
  List.hd (Utf8.decode (String.sub s (!i - len) (1 + len)))

let codepoint lexbuf s i =
  let u =
    if s.[!i] >= '\x80' then Utf8.encode [utf8 s i] else
    if s.[!i] <> '\\' then Utf8.encode [Char.code s.[!i]] else
    match (incr i; s.[!i]) with
    | 'n' -> Utf8.encode [Char.code '\n']
    | 'r' -> Utf8.encode [Char.code '\r']
    | 't' -> Utf8.encode [Char.code '\t']
    | '\\' -> Utf8.encode [Char.code '\\']
    | '\'' -> Utf8.encode [Char.code '\'']
    | '\"' -> Utf8.encode [Char.code '\"']
    | 'u' ->
      let j = !i + 2 in
      i := String.index_from s j '}';
      Utf8.encode [
        try
          let n = int_of_string ("0x" ^ String.sub s j (!i - j)) in
          if 0 <= n && n < 0xD800 || 0xE000 <= n && n < 0x110000 then n else raise (Failure "")
        with Failure _ -> error lexbuf "unicode escape out of range"
      ]
    | h ->
      incr i;
      let b = int_of_string ("0x" ^ String.make 1 h ^ String.make 1 s.[!i]) in
      String.of_seq (Seq.return (Char.chr b))
  in incr i; u

let text lexbuf s =
  let b = Buffer.create (String.length s) in
  let i = ref 1 in
  while !i < String.length s - 1 do
    if s.[!i] = '\n' then Lexing.new_line lexbuf;
    let bs = codepoint lexbuf s i in
    Buffer.add_substring b bs 0 (String.length bs)
  done;
  Buffer.contents b

let char lexbuf s =
  let t = text lexbuf s in
  try
    match Utf8.decode t with
    | [n] -> n
    | [] -> error lexbuf "empty character literal"
    | _ -> error lexbuf "overlong character literal"
  with Lib.Utf8.Utf8 ->
    error lexbuf "invalid utf8 in character literal"
}

let sign = '+' | '-'
let digit = ['0'-'9']
let hexdigit = ['0'-'9''a'-'f''A'-'F']
let num = digit ('_'? digit)*
let hexnum = hexdigit ('_'? hexdigit)*

let letter = ['a'-'z''A'-'Z']
let symbol =
  ['+''-''*''/''\\''^''~''=''<''>''!''?''@''#''$''%''&''|'':''`''.''\'']

let space = [' ''\t''\n''\r']
let ascii = ['\x00'-'\x7f']
let ascii_no_nl = ['\x00'-'\x09''\x0b'-'\x7f']
let utf8cont = ['\x80'-'\xbf']
let utf8enc =
    ['\xc2'-'\xdf'] utf8cont
  | ['\xe0'] ['\xa0'-'\xbf'] utf8cont
  | ['\xed'] ['\x80'-'\x9f'] utf8cont
  | ['\xe1'-'\xec''\xee'-'\xef'] utf8cont utf8cont
  | ['\xf0'] ['\x90'-'\xbf'] utf8cont utf8cont
  | ['\xf4'] ['\x80'-'\x8f'] utf8cont utf8cont
  | ['\xf1'-'\xf3'] utf8cont utf8cont utf8cont
let utf8 = ascii | utf8enc
let utf8_no_nl = ascii_no_nl | utf8enc

let byte = '\\'hexdigit hexdigit
let escape = ['n''r''t''\\''\'''\"']
let character =
    [^'"''\\''\x00'-'\x1f''\x7f'-'\xff']
  | utf8enc
  | byte
  | '\\'escape
  | "\\u{" hexnum '}'
  | '\n'

let nat = num | "0x" hexnum
let frac = num
let hexfrac = hexnum
let float =
    num '.' frac?
  | num ('.' frac?)? ('e' | 'E') sign? num
  | "0x" hexnum '.' hexfrac?
  | "0x" hexnum ('.' hexfrac?)? ('p' | 'P') sign? num
let char = '\'' (character | byte+) '\''
let text = '"' character* '"'
let id = ((letter  | '_') ((letter | digit | '_')*))
let privileged_id = "@" id
let num_dot_ident =
    num '.' id

let reserved = ([^'\"''('')'';'] # space)+  (* hack for table size *)

rule token mode = parse
  | "(" { LPAR }
  | ")" { RPAR }
  | "[" { LBRACKET }
  | "]" { RBRACKET }
  | "{" { LCURLY }
  | "}" { RCURLY }
  | ";" { SEMICOLON }
  | "," { COMMA }
  | ":" { COLON }
  | "<:" { SUB }
  | "." { DOT }
  | "?" { QUEST }
  | "!" { BANG }
  | "=" { EQ }
  | "<" { LT }
  | ">" { GT }
  | "+" { ADDOP }
  | "-" { SUBOP }
  | "*" { MULOP }
  | "/" { DIVOP }
  | "%" { MODOP }
  | "**" { POWOP }
  | "&" { ANDOP }
  | "|" { OROP }
  | "|>" { PIPE }
  | "^" { XOROP }
  | "<<" { SHLOP }
  | "<<>" { ROTLOP }
  | "<>>" { ROTROP }
  | "+%" { WRAPADDOP }
  | "-%" { WRAPSUBOP }
  | "*%" { WRAPMULOP }
  | "**%" { WRAPPOWOP }
  | "#" { HASH }

  | "==" { EQOP }
  | "!=" { NEQOP }
  | ">=" { GEOP }
  | "<=" { LEOP }
  | ":=" { ASSIGN }

  | "+=" { PLUSASSIGN }
  | "-=" { MINUSASSIGN }
  | "*=" { MULASSIGN }
  | "/=" { DIVASSIGN }
  | "%=" { MODASSIGN }
  | "**=" { POWASSIGN }
  | "&=" { ANDASSIGN }
  | "|=" { ORASSIGN }
  | "^=" { XORASSIGN }
  | "<<=" { SHLASSIGN }
  | ">>=" { SHRASSIGN }
  | "<<>=" { ROTLASSIGN }
  | "<>>=" { ROTRASSIGN }
  | "+%=" { WRAPADDASSIGN }
  | "-%=" { WRAPSUBASSIGN }
  | "*%=" { WRAPMULASSIGN }
  | "**%=" { WRAPPOWASSIGN }
  | "#=" { CATASSIGN }
  | "->" { ARROW }
  | "_" { UNDERSCORE }

  | '.' (num as s) { DOT_NUM s }
  | nat as s { NAT s }
  | num_dot_ident as s {
      match Lib.String.split s '.' with
      | [n; id] -> NUM_DOT_ID (n, id)
      | _ -> assert false
    }
  | float as s { FLOAT s }
  | char as s { CHAR (char lexbuf s) }
  | text as s { TEXT (text lexbuf s) }
  | '"'character*eof
    { error lexbuf "unclosed text literal" }
  | '"'character*['\x00'-'\x09''\x0b'-'\x1f''\x7f']
    { error lexbuf "illegal control character in text literal" }
  | '"'character*'\\'_
    { error_nest (Lexing.lexeme_end_p lexbuf) lexbuf "illegal escape" }

  (* If you add keywords, please also update
     - src/idllib/escape.ml
     - emacs/motoko-mode.el
     - src/gen-grammar/grammar.sed
  *)
  | "actor" { ACTOR }
  | "and" { AND }
  | "async" { ASYNC }
  | "async*" { ASYNCSTAR }
  | "assert" { ASSERT }
  | "await" { AWAIT }
  | "await*" { AWAITSTAR }
  | "await?" { AWAITQUEST }
  | "break" { BREAK }
  | "case" { CASE }
  | "catch" { CATCH }
  | "class" { CLASS }
  | "composite" { COMPOSITE }
  | "continue" { CONTINUE }
  | "debug" { DEBUG }
  | "debug_show" { DEBUG_SHOW }
  | "do" { DO }
  | "else" { ELSE }
  | "false" { BOOL false }
  | "finally" { FINALLY }
  | "flexible" { FLEXIBLE }
  | "for" { FOR }
  | "from_candid" { FROM_CANDID }
  | "func" { FUNC }
  | "if" { IF }
  | "ignore" { IGNORE }
  | "in" { IN }
  | "implicit" { IMPLICIT }
  | "import" { IMPORT }
  | "include" { INCLUDE }
  | "mixin" { MIXIN }
  | "module" { MODULE }
  | "not" { NOT }
  | "null" { NULL }
  | "object" { OBJECT }
  | "or" { OR }
  | "label" { LABEL }
  | "let" { LET }
  | "loop" { LOOP }
  | "persistent" { PERSISTENT}
  | "private" { PRIVATE }
  | "public" { PUBLIC }
  | "query" { QUERY }
  | "return" { RETURN }
  | "shared" { SHARED }
  | "stable" { STABLE }
  | "switch" { SWITCH }
  | "system" { SYSTEM }
  | "transient" { TRANSIENT }
  | "try" { TRY }
  | "throw" { THROW }
  | "to_candid" { TO_CANDID }
  | "true" { BOOL true }
  | "type" { TYPE }
  | "var" { VAR }
  | "weak" { WEAK }
  | "while" { WHILE }
  | "with" { WITH }

  | "prim" as s { if mode.privileged then PRIM else ID s }
  | id as s { ID s }
  | privileged_id as s { if mode.privileged then ID s else error lexbuf "privileged identifier" }

  | "//"utf8_no_nl* as s { COMMENT s }
  | "/*" as s {
    let buf = Buffer.create 100 in
    Buffer.add_string buf s;
    (comment buf (Lexing.lexeme_start_p lexbuf) lexbuf);
    COMMENT (Buffer.contents buf)
  }
  | '\t'+ as t { TAB (String.length t) }
  | ' '+ as s { let len = String.length s in if len = 1 then SINGLESPACE else SPACE len }
  | "\r\n" { Lexing.new_line lexbuf; LINEFEED CRLF }
  | '\r' { Lexing.new_line lexbuf; LINEFEED CRLF }
  | '\n' { Lexing.new_line lexbuf; LINEFEED LF }
  | eof { EOF }

  | utf8 { error lexbuf "malformed operator" }
  | _ { error lexbuf "malformed UTF-8 encoding" }

and comment buf start = parse
  | "*/" as s { Buffer.add_string buf s }
  | "/*" as s {
    Buffer.add_string buf s;
    comment buf (Lexing.lexeme_start_p lexbuf) lexbuf;
    comment buf start lexbuf
  }
  | '\n' as s {
    Lexing.new_line lexbuf;
    Buffer.add_char buf s;
    comment buf start lexbuf
  }
  | eof { error_nest start lexbuf "unclosed comment" }
  | utf8 as s {
    Buffer.add_string buf s;
    comment buf start lexbuf
  }
  | _ { error lexbuf "malformed UTF-8 encoding" }
