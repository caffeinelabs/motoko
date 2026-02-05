open Mo_def
open Mo_types

open Type
open Scope

val initial_scope : scope

val infer_prog
  :  ?enable_type_recovery:bool
  -> scope
  -> string option
  -> Async_cap.async_cap
  -> Syntax.prog
  -> (typ * scope) Diag.result

val check_lib : scope -> string option -> Syntax.lib -> scope Diag.result
val check_actors : ?check_actors:bool -> scope -> Syntax.prog list -> unit Diag.result

val check_stab_sig : scope -> Syntax.stab_sig -> Type.stab_sig Diag.result

type contextual_dot_suggestion =
  { module_url : lab;
    func_name : lab;
    func_ty : typ;
  }

val contextual_dot_suggestions : lib_env -> typ -> contextual_dot_suggestion list

val contextual_dot_module : Syntax.exp -> (string * string) option
