(**
  This module exists so it can be included by lexer.ml. This way
  source_lexer.ml can use these definitions but stay internal to
  lexer.ml.
 *)

open Source

type mode = {
  privileged : bool;
  verification : bool;
}

let mode : mode = {
  privileged = Option.is_some (Sys.getenv_opt "MOC_UNLOCK_PRIM");
  verification = Option.is_some (Sys.getenv_opt "MOC_UNLOCK_VERIFICATION");
}

let mode_priv : mode = { mode with privileged = true }
let mode_verification : mode = { mode with verification = true }


exception Error of region * string

let convert_pos pos =
  Lexing.{
      file = pos.pos_fname;
      line = pos.pos_lnum;
      column = pos.pos_cnum - pos.pos_bol
  }

