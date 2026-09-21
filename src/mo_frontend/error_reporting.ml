open Parser
open Parser.MenhirInterpreter

(* In order to submit artificial tokens to the parser, we need a function that
   converts a terminal symbol to a (dummy) token. Unfortunately, we cannot (in
   general) auto-generate this code, because it requires making up semantic
   values of arbitrary OCaml types. *)

let terminal2token (type a) (symbol : a terminal) : token =
  match symbol with
      | T_error -> assert false
      | T_XOROP -> XOROP
      | T_XORASSIGN -> XORASSIGN
      | T_WHILE -> WHILE
      | T_VAR -> VAR
      | T_SHROP -> SHROP
      | T_SHRASSIGN -> SHRASSIGN
      | T_UNDERSCORE -> UNDERSCORE
      | T_COMPOSITE -> COMPOSITE
      | T_TYPE -> TYPE
      | T_TRANSIENT -> TRANSIENT
      | T_TRY -> TRY
      | T_THROW -> THROW
      | T_FINALLY -> FINALLY
      | T_TEXT -> TEXT "..."
      | T_SWITCH -> SWITCH
      | T_SUBOP -> SUBOP
      | T_SUB -> SUB
      | T_SHLOP -> SHLOP
      | T_SHLASSIGN -> SHLASSIGN
      | T_SHARED -> SHARED
      | T_SEMICOLON_EOL -> SEMICOLON_EOL
      | T_SEMICOLON -> SEMICOLON
      | T_STABLE -> STABLE
      | T_SYSTEM -> SYSTEM
      | T_WITH -> WITH
      | T_RPAR -> RPAR
      | T_ROTROP -> ROTROP
      | T_ROTRASSIGN -> ROTRASSIGN
      | T_ROTLOP -> ROTLOP
      | T_ROTLASSIGN -> ROTLASSIGN
      | T_RETURN -> RETURN
      | T_RCURLY -> RCURLY
      | T_RBRACKET -> RBRACKET
      | T_QUEST -> QUEST
      | T_BANG -> BANG
      | T_QUERY -> QUERY
      | T_PUBLIC -> PUBLIC
      | T_PERSISTENT -> PERSISTENT
      | T_PRIVATE -> PRIVATE
      | T_PRIM -> PRIM
      | T_POWOP -> POWOP
      | T_POWASSIGN -> POWASSIGN
      | T_PLUSASSIGN -> PLUSASSIGN
      | T_OROP -> OROP
      | T_ORASSIGN -> ORASSIGN
      | T_OR -> OR
      | T_OBJECT -> OBJECT
      | T_NULL -> NULL
      | T_NULLCOALESCE -> NULLCOALESCE
      | T_NOT -> NOT
      | T_NEQOP -> NEQOP
      | T_NAT -> NAT "<nat>"
      | T_NUM_DOT_ID -> NUM_DOT_ID ("<num>","<id>")
      | T_MULOP -> MULOP
      | T_MULASSIGN -> MULASSIGN
      | T_MODULE -> MODULE
      | T_INCLUDE -> INCLUDE
      | T_MIXIN -> MIXIN
      | T_MODOP -> MODOP
      | T_MODASSIGN -> MODASSIGN
      | T_MINUSASSIGN -> MINUSASSIGN
      | T_LTOP -> LTOP
      | T_LT -> LT
      | T_LPAR -> LPAR
      | T_LOOP -> LOOP
      | T_LET -> LET
      | T_LEOP -> LEOP
      | T_LCURLY -> LCURLY
      | T_LBRACKET -> LBRACKET
      | T_LABEL -> LABEL
      | T_IN -> IN
      | T_IMPORT -> IMPORT
      | T_IGNORE -> IGNORE
      | T_IF -> IF
      | T_ID -> ID "<id>"
      | T_IMPLICIT -> IMPLICIT
      | T_HASH -> HASH
      | T_GTOP -> GTOP
      | T_GT -> GT
      | T_GEOP -> GEOP
      | T_FUNC -> FUNC
      | T_FOR -> FOR
      | T_FLEXIBLE -> FLEXIBLE
      | T_FLOAT -> FLOAT "<float>"
      | T_EQOP -> EQOP
      | T_EQ -> EQ
      | T_EOF -> EOF
      | T_ELSE -> ELSE
      | T_DOT_NUM -> DOT_NUM "<num>"
      | T_DOT -> DOT
      | T_DO -> DO
      | T_DIVOP -> DIVOP
      | T_DIVASSIGN -> DIVASSIGN
      | T_DISALLOWED -> DISALLOWED
      | T_DEBUG_SHOW -> DEBUG_SHOW
      | T_TO_CANDID -> TO_CANDID
      | T_FROM_CANDID -> FROM_CANDID
      | T_DEBUG -> DEBUG
      | T_CONTINUE -> CONTINUE
      | T_COMMA -> COMMA
      | T_COLON -> COLON
      | T_CLASS -> CLASS
      | T_CHAR -> CHAR 0
      | T_CATCH -> CATCH
      | T_CATASSIGN -> CATASSIGN
      | T_CASE -> CASE
      | T_BREAK -> BREAK
      | T_BOOL -> BOOL false
      | T_AWAIT -> AWAIT
      | T_AWAITSTAR -> AWAITSTAR
      | T_AWAITQUEST -> AWAITQUEST
      | T_ASYNC -> ASYNC
      | T_ASYNCSTAR -> ASYNCSTAR
      | T_ASSIGN -> ASSIGN
      | T_ASSERT -> ASSERT
      | T_ARROW -> ARROW
      | T_ANDOP -> ANDOP
      | T_ANDASSIGN -> ANDASSIGN
      | T_AND -> AND
      | T_ADDOP -> ADDOP
      | T_ACTOR -> ACTOR
      | T_WRAPADDOP -> WRAPADDOP
      | T_WRAPSUBOP -> WRAPSUBOP
      | T_WRAPMULOP -> WRAPMULOP
      | T_WRAPPOWOP -> WRAPPOWOP
      | T_WRAPADDASSIGN -> WRAPADDASSIGN
      | T_WRAPSUBASSIGN -> WRAPSUBASSIGN
      | T_WRAPMULASSIGN -> WRAPMULASSIGN
      | T_WRAPPOWASSIGN -> WRAPPOWASSIGN
      | T_PIPE -> PIPE
      | T_WEAK -> WEAK
