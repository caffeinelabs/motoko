open Trivia

type token =
  | EOF
  | DISALLOWED
  | LET
  | VAR
  | LPAR
  | RPAR
  | LBRACKET
  | RBRACKET
  | LCURLY
  | RCURLY
  | AWAIT
  | AWAITSTAR
  | AWAITQUEST
  | ASYNC
  | ASYNCSTAR
  | BREAK
  | CASE
  | CATCH
  | CONTINUE
  | LABEL
  | DEBUG
  | DO
  | FINALLY
  | FLEXIBLE
  | IF
  | IGNORE
  | IN
  | IMPLICIT
  | ELSE
  | SWITCH
  | LOOP
  | WHILE
  | FOR
  | RETURN
  | SYSTEM
  | STABLE
  | TRANSIENT
  | TRY
  | THROW
  | WITH
  | ARROW
  | ASSIGN
  | FUNC
  | TYPE
  | OBJECT
  | ACTOR
  | CLASS
  | PERSISTENT
  | PUBLIC
  | PRIVATE
  | SHARED
  | QUERY
  | SEMICOLON
  | COMMA
  | COLON
  | SUB
  | DOT
  | QUEST
  | BANG
  | AND
  | OR
  | NOT
  | IMPORT
  | INCLUDE
  | MIXIN
  | MODULE
  | DEBUG_SHOW
  | TO_CANDID
  | FROM_CANDID
  | ASSERT
  | ADDOP
  | SUBOP
  | MULOP
  | DIVOP
  | MODOP
  | POWOP
  | WRAPADDOP
  | WRAPSUBOP
  | WRAPMULOP
  | WRAPPOWOP
  | WRAPADDASSIGN
  | WRAPSUBASSIGN
  | WRAPMULASSIGN
  | WRAPPOWASSIGN
  | ANDOP
  | OROP
  | XOROP
  | SHLOP
  | ROTLOP
  | ROTROP
  | EQOP
  | NEQOP
  | LEOP
  | GEOP
  | HASH
  | EQ
  | LT
  | GT
  | PLUSASSIGN
  | MINUSASSIGN
  | MULASSIGN
  | DIVASSIGN
  | MODASSIGN
  | POWASSIGN
  | CATASSIGN
  | ANDASSIGN
  | ORASSIGN
  | XORASSIGN
  | SHLASSIGN
  | SHRASSIGN
  | ROTLASSIGN
  | ROTRASSIGN
  | NULL
  | NULLCOALESCE
  | DOT_NUM of string
  | NUM_DOT_ID of string * string
  | NAT of string
  | FLOAT of string
  | CHAR of Mo_values.Value.unicode
  | BOOL of bool
  | ID of string
  | TEXT of string
  | PRIM
  | PIPE
  | UNDERSCORE
  | WEAK
  | COMPOSITE
  (* Trivia *)
  | LINEFEED of line_feed
  | SINGLESPACE
  | SPACE of int
  | TAB of int (* shudders *)
  | COMMENT of string

let to_parser_token :
    token -> (Parser.token, line_feed trivia) result = function
  | EOF -> Ok Parser.EOF
  | DISALLOWED -> Ok Parser.DISALLOWED
  | LET -> Ok Parser.LET
  | VAR -> Ok Parser.VAR
  | LPAR -> Ok Parser.LPAR
  | RPAR -> Ok Parser.RPAR
  | LBRACKET -> Ok Parser.LBRACKET
  | RBRACKET -> Ok Parser.RBRACKET
  | LCURLY -> Ok Parser.LCURLY
  | RCURLY -> Ok Parser.RCURLY
  | AWAIT -> Ok Parser.AWAIT
  | AWAITSTAR -> Ok Parser.AWAITSTAR
  | AWAITQUEST -> Ok Parser.AWAITQUEST
  | ASYNC -> Ok Parser.ASYNC
  | ASYNCSTAR -> Ok Parser.ASYNCSTAR
  | BREAK -> Ok Parser.BREAK
  | CASE -> Ok Parser.CASE
  | CATCH -> Ok Parser.CATCH
  | CONTINUE -> Ok Parser.CONTINUE
  | LABEL -> Ok Parser.LABEL
  | DO -> Ok Parser.DO
  | DEBUG -> Ok Parser.DEBUG
  | FLEXIBLE -> Ok Parser.FLEXIBLE
  | IF -> Ok Parser.IF
  | IGNORE -> Ok Parser.IGNORE
  | IN -> Ok Parser.IN
  | IMPLICIT -> Ok Parser.IMPLICIT
  | ELSE -> Ok Parser.ELSE
  | SWITCH -> Ok Parser.SWITCH
  | LOOP -> Ok Parser.LOOP
  | WHILE -> Ok Parser.WHILE
  | FOR -> Ok Parser.FOR
  | RETURN -> Ok Parser.RETURN
  | TRANSIENT -> Ok Parser.TRANSIENT
  | TRY -> Ok Parser.TRY
  | THROW -> Ok Parser.THROW
  | FINALLY -> Ok Parser.FINALLY
  | WITH -> Ok Parser.WITH
  | ARROW -> Ok Parser.ARROW
  | ASSIGN -> Ok Parser.ASSIGN
  | FUNC -> Ok Parser.FUNC
  | TYPE -> Ok Parser.TYPE
  | OBJECT -> Ok Parser.OBJECT
  | ACTOR -> Ok Parser.ACTOR
  | CLASS -> Ok Parser.CLASS
  | PERSISTENT -> Ok Parser.PERSISTENT
  | PUBLIC -> Ok Parser.PUBLIC
  | PRIVATE -> Ok Parser.PRIVATE
  | SHARED -> Ok Parser.SHARED
  | STABLE -> Ok Parser.STABLE
  | SYSTEM -> Ok Parser.SYSTEM
  | QUERY -> Ok Parser.QUERY
  | SEMICOLON -> Ok Parser.SEMICOLON
  | COMMA -> Ok Parser.COMMA
  | COLON -> Ok Parser.COLON
  | SUB -> Ok Parser.SUB
  | DOT -> Ok Parser.DOT
  | QUEST -> Ok Parser.QUEST
  | BANG -> Ok Parser.BANG
  | AND -> Ok Parser.AND
  | OR -> Ok Parser.OR
  | NOT -> Ok Parser.NOT
  | IMPORT -> Ok Parser.IMPORT
  | INCLUDE -> Ok Parser.INCLUDE
  | MIXIN -> Ok Parser.MIXIN
  | MODULE -> Ok Parser.MODULE
  | DEBUG_SHOW -> Ok Parser.DEBUG_SHOW
  | TO_CANDID -> Ok Parser.TO_CANDID
  | FROM_CANDID -> Ok Parser.FROM_CANDID
  | ASSERT -> Ok Parser.ASSERT
  | ADDOP -> Ok Parser.ADDOP
  | SUBOP -> Ok Parser.SUBOP
  | MULOP -> Ok Parser.MULOP
  | DIVOP -> Ok Parser.DIVOP
  | MODOP -> Ok Parser.MODOP
  | POWOP -> Ok Parser.POWOP
  | WRAPADDOP -> Ok Parser.WRAPADDOP
  | WRAPSUBOP -> Ok Parser.WRAPSUBOP
  | WRAPMULOP -> Ok Parser.WRAPMULOP
  | WRAPPOWOP -> Ok Parser.WRAPPOWOP
  | WRAPADDASSIGN -> Ok Parser.WRAPADDASSIGN
  | WRAPSUBASSIGN -> Ok Parser.WRAPSUBASSIGN
  | WRAPMULASSIGN -> Ok Parser.WRAPMULASSIGN
  | WRAPPOWASSIGN -> Ok Parser.WRAPPOWASSIGN
  | ANDOP -> Ok Parser.ANDOP
  | OROP -> Ok Parser.OROP
  | XOROP -> Ok Parser.XOROP
  | SHLOP -> Ok Parser.SHLOP
  | ROTLOP -> Ok Parser.ROTLOP
  | ROTROP -> Ok Parser.ROTROP
  | EQOP -> Ok Parser.EQOP
  | NEQOP -> Ok Parser.NEQOP
  | LEOP -> Ok Parser.LEOP
  | GEOP -> Ok Parser.GEOP
  | HASH -> Ok Parser.HASH
  | EQ -> Ok Parser.EQ
  | LT -> Ok Parser.LT
  | GT -> Ok Parser.GT
  | PLUSASSIGN -> Ok Parser.PLUSASSIGN
  | MINUSASSIGN -> Ok Parser.MINUSASSIGN
  | MULASSIGN -> Ok Parser.MULASSIGN
  | DIVASSIGN -> Ok Parser.DIVASSIGN
  | MODASSIGN -> Ok Parser.MODASSIGN
  | POWASSIGN -> Ok Parser.POWASSIGN
  | CATASSIGN -> Ok Parser.CATASSIGN
  | ANDASSIGN -> Ok Parser.ANDASSIGN
  | ORASSIGN -> Ok Parser.ORASSIGN
  | XORASSIGN -> Ok Parser.XORASSIGN
  | SHLASSIGN -> Ok Parser.SHLASSIGN
  | SHRASSIGN -> Ok Parser.SHRASSIGN
  | ROTLASSIGN -> Ok Parser.ROTLASSIGN
  | ROTRASSIGN -> Ok Parser.ROTRASSIGN
  | NULL -> Ok Parser.NULL
  | NULLCOALESCE -> Ok Parser.NULLCOALESCE
  | NUM_DOT_ID (ns, id) -> Ok (Parser.NUM_DOT_ID (ns, id))
  | DOT_NUM s -> Ok (Parser.DOT_NUM s)
  | NAT s -> Ok (Parser.NAT s)
  | FLOAT s -> Ok (Parser.FLOAT s)
  | CHAR u -> Ok (Parser.CHAR u)
  | BOOL b -> Ok (Parser.BOOL b)
  | ID s -> Ok (Parser.ID s)
  | TEXT s -> Ok (Parser.TEXT s)
  | PRIM -> Ok Parser.PRIM
  | UNDERSCORE -> Ok Parser.UNDERSCORE
  | COMPOSITE -> Ok Parser.COMPOSITE
  | PIPE -> Ok Parser.PIPE
  | WEAK -> Ok Parser.WEAK
  (*Trivia *)
  | SINGLESPACE -> Error (Space 1)
  | SPACE n -> Error (Space n)
  | LINEFEED lf -> Error (Line lf)
  | TAB n -> Error (Tab n)
  | COMMENT c -> Error (Comment c)

let string_of_parser_token = function
  | Parser.EOF -> "EOF"
  | Parser.DISALLOWED -> "DISALLOWED"
  | Parser.LET -> "LET"
  | Parser.VAR -> "VAR"
  | Parser.LPAR -> "LPAR"
  | Parser.RPAR -> "RPAR"
  | Parser.LBRACKET -> "LBRACKET"
  | Parser.RBRACKET -> "RBRACKET"
  | Parser.LCURLY -> "LCURLY"
  | Parser.RCURLY -> "RCURLY"
  | Parser.AWAIT -> "AWAIT"
  | Parser.AWAITSTAR -> "AWAIT*"
  | Parser.AWAITQUEST -> "AWAIT?"
  | Parser.ASYNC -> "ASYNC"
  | Parser.ASYNCSTAR -> "ASYNC*"
  | Parser.BREAK -> "BREAK"
  | Parser.CASE -> "CASE"
  | Parser.CATCH -> "CATCH"
  | Parser.CONTINUE -> "CONTINUE"
  | Parser.LABEL -> "LABEL"
  | Parser.DEBUG -> "DEBUG"
  | Parser.DO -> "DO"
  | Parser.FLEXIBLE -> "FLEXIBLE"
  | Parser.IF -> "IF"
  | Parser.IGNORE -> "IGNORE"
  | Parser.IN -> "IN"
  | Parser.IMPLICIT -> "IMPLICIT"
  | Parser.ELSE -> "ELSE"
  | Parser.SWITCH -> "SWITCH"
  | Parser.LOOP -> "LOOP"
  | Parser.WHILE -> "WHILE"
  | Parser.FOR -> "FOR"
  | Parser.RETURN -> "RETURN"
  | Parser.TRY -> "TRY"
  | Parser.THROW -> "THROW"
  | Parser.FINALLY -> "FINALLY"
  | Parser.WITH -> "WITH"
  | Parser.ARROW -> "ARROW"
  | Parser.ASSIGN -> "ASSIGN"
  | Parser.FUNC -> "FUNC"
  | Parser.TYPE -> "TYPE"
  | Parser.OBJECT -> "OBJECT"
  | Parser.ACTOR -> "ACTOR"
  | Parser.CLASS -> "CLASS"
  | Parser.PERSISTENT -> "PERSISTENT"
  | Parser.PUBLIC -> "PUBLIC"
  | Parser.PRIVATE -> "PRIVATE"
  | Parser.SHARED -> "SHARED"
  | Parser.STABLE -> "STABLE"
  | Parser.SYSTEM -> "SYSTEM"
  | Parser.TRANSIENT -> "TRANSIENT"
  | Parser.QUERY -> "QUERY"
  | Parser.SEMICOLON -> "SEMICOLON"
  | Parser.SEMICOLON_EOL -> "SEMICOLON_EOL"
  | Parser.COMMA -> "COMMA"
  | Parser.COLON -> "COLON"
  | Parser.SUB -> "SUB"
  | Parser.DOT -> "DOT"
  | Parser.QUEST -> "QUEST"
  | Parser.BANG -> "BANG"
  | Parser.AND -> "AND"
  | Parser.OR -> "OR"
  | Parser.NOT -> "NOT"
  | Parser.IMPORT -> "IMPORT"
  | Parser.INCLUDE -> "INCLUDE"
  | Parser.MIXIN -> "MIXIN"
  | Parser.MODULE -> "MODULE"
  | Parser.DEBUG_SHOW -> "DEBUG_SHOW"
  | Parser.TO_CANDID -> "TO_CANDID"
  | Parser.FROM_CANDID -> "FROM_CANDID"
  | Parser.ASSERT -> "ASSERT"
  | Parser.ADDOP -> "ADDOP"
  | Parser.SUBOP -> "SUBOP"
  | Parser.MULOP -> "MULOP"
  | Parser.DIVOP -> "DIVOP"
  | Parser.MODOP -> "MODOP"
  | Parser.POWOP -> "POWOP"
  | Parser.WRAPADDOP -> "WRAPADDOP"
  | Parser.WRAPSUBOP -> "WRAPSUBOP"
  | Parser.WRAPMULOP -> "WRAPMULOP"
  | Parser.WRAPPOWOP -> "WRAPPOWOP"
  | Parser.WRAPADDASSIGN -> "WRAPADDASSIGN"
  | Parser.WRAPSUBASSIGN -> "WRAPSUBASSIGN"
  | Parser.WRAPMULASSIGN -> "WRAPMULASSIGN"
  | Parser.WRAPPOWASSIGN -> "WRAPPOWASSIGN"
  | Parser.ANDOP -> "ANDOP"
  | Parser.OROP -> "OROP"
  | Parser.XOROP -> "XOROP"
  | Parser.SHLOP -> "SHLOP"
  | Parser.SHROP -> "SHROP"
  | Parser.ROTLOP -> "ROTLOP"
  | Parser.ROTROP -> "ROTROP"
  | Parser.EQOP -> "EQOP"
  | Parser.NEQOP -> "NEQOP"
  | Parser.LEOP -> "LEOP"
  | Parser.LTOP -> "LTOP"
  | Parser.GTOP -> "GTOP"
  | Parser.GEOP -> "GEOP"
  | Parser.HASH -> "HASH"
  | Parser.EQ -> "EQ"
  | Parser.LT -> "LT"
  | Parser.GT -> "GT"
  | Parser.PLUSASSIGN -> "PLUSASSIGN"
  | Parser.MINUSASSIGN -> "MINUSASSIGN"
  | Parser.MULASSIGN -> "MULASSIGN"
  | Parser.DIVASSIGN -> "DIVASSIGN"
  | Parser.MODASSIGN -> "MODASSIGN"
  | Parser.POWASSIGN -> "POWASSIGN"
  | Parser.CATASSIGN -> "CATASSIGN"
  | Parser.ANDASSIGN -> "ANDASSIGN"
  | Parser.ORASSIGN -> "ORASSIGN"
  | Parser.XORASSIGN -> "XORASSIGN"
  | Parser.SHLASSIGN -> "SHLASSIGN"
  | Parser.SHRASSIGN -> "SHRASSIGN"
  | Parser.ROTLASSIGN -> "ROTLASSIGN"
  | Parser.ROTRASSIGN -> "ROTRASSIGN"
  | Parser.NULL -> "NULL"
  | Parser.NUM_DOT_ID _ -> "NUM_DOT_IDENT of string * string"
  | Parser.DOT_NUM _ -> "DOT_NUM of string"
  | Parser.NAT _ -> "NAT of string"
  | Parser.FLOAT _ -> "FLOAT of string"
  | Parser.CHAR _ -> "CHAR of Mo_values.Value.unicode"
  | Parser.BOOL _ -> "BOOL of bool"
  | Parser.ID _ -> "ID of string"
  | Parser.TEXT _ -> "TEXT of string"
  | Parser.PRIM -> "PRIM"
  | Parser.UNDERSCORE -> "UNDERSCORE"
  | Parser.COMPOSITE -> "COMPOSITE"
  | Parser.PIPE -> "PIPE"
  | Parser.WEAK -> "WEAK"
  | Parser.NULLCOALESCE -> "NULLCOALESCE"

let is_lineless_trivia : token -> void trivia option = function
  | SINGLESPACE -> Some (Space 1)
  | SPACE n -> Some (Space n)
  | TAB n -> Some (Tab n)
  | COMMENT c -> Some (Comment c)
  | _ -> None

let is_trivia : token -> line_feed trivia option = function
  | LINEFEED lf -> Some (Line lf)
  | t -> Option.map (map_trivia absurd) (is_lineless_trivia t)

let is_whitespace : 'a trivia -> bool = function
  | Space _ | Tab _ | Line _ -> true
  | Comment _ -> false

let is_line_feed : token -> bool = function
  | LINEFEED _ -> true
  | _ -> false

type annotation = {
  range : Lexing.position * Lexing.position;
  leading_trivia : line_feed trivia list;
  trailing_trivia : void trivia list;
}

let dummy_annotation = {
   range = Lexing.dummy_pos, Lexing.dummy_pos;
   leading_trivia = [];
   trailing_trivia = [];
}
