open Mo_def
open Mo_values
open Mo_types
open Source

module V = Value
module T = Type

type flags =
  { trace : bool;
    print_depth : int
  }

type scope =
  { val_env: V.def V.Env.t;
    lib_env: V.value V.Env.t;
  }

val empty_scope : scope
val adjoin_scope : scope -> scope -> scope

val step_limit : int ref

exception Trap of region * string

val interpret_prog : flags -> scope -> Syntax.prog -> (V.value * scope) option
val interpret_lib : flags -> scope -> Syntax.lib -> scope
