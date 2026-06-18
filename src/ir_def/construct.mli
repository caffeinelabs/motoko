open Mo_types

open Ir
open Type

(* A miscellany of helpers to construct typed terms from typed terms *)

(* For convenience, fresh identifiers are returned as expressions, and binders
   take expressions (that must be variables) as arguments.
   This makes code transformations easier to write and read,
   at the loss of some precision in OCaml typing.
*)


(* Field names *)

val nameN : string -> Type.lab
val nextN : Type.lab

(* Identifiers *)

type var

val var : id -> typ -> var
val id_of_var : var -> id
val typ_of_var : var -> typ
val arg_of_var : var -> arg
val var_of_arg : arg -> var

val fresh_id : string -> unit -> id
val fresh_var : string -> typ -> var
val fresh_vars : string -> typ list -> var list

(* type arguments *)

val typ_arg : con -> bind_sort -> typ -> typ_bind

(* Patterns *)

val varP : var -> pat
val tupP :  pat list -> pat
val wildP : pat

val seqP : pat list -> pat

(* Expressions *)

val varE : var -> exp
val primE : Ir.prim -> exp list -> exp
val selfRefE : typ -> exp
val assertE : exp -> exp
val asyncE : async_sort -> typ_bind -> exp -> typ -> exp
val awaitE : await_sort -> exp -> exp
val cps_asyncE : async_sort -> typ -> typ -> exp -> exp
val cps_awaitE : await_sort -> typ -> exp -> exp -> exp
val ic_replyE : typ list -> exp -> exp
val ic_rejectE : exp -> exp
val ic_callE : exp -> exp -> exp -> exp -> exp -> exp
val ic_call_rawE : exp -> exp -> exp -> exp -> exp -> exp -> exp
val projE : exp -> int -> exp
val optE : exp -> exp
val tagE : id -> exp -> exp
val thenE : exp -> exp -> exp
val blockE : dec list -> exp -> exp
val let_else_switch : pat -> exp -> exp -> exp
val natE : Mo_values.Numerics.Nat.t -> exp
val intE : Mo_values.Numerics.Int.t -> exp
val nat32E : Mo_values.Numerics.Nat32.t -> exp
val nat64E : Mo_values.Numerics.Nat64.t -> exp
val textE : string -> exp
val blobE : string -> exp
val letE : var -> exp -> exp -> exp
val ignoreE : exp -> exp

val unitE : unit -> exp
val boolE : bool -> exp
val nullE : unit -> exp

val funcE : string -> func_sort -> control ->
  typ_bind list -> arg list -> typ list -> exp ->
  exp
val callE : exp -> typ list -> exp -> exp

val ifE : exp -> exp -> exp -> exp
val dotE : exp -> Type.lab -> typ -> exp
val switch_optE : exp -> exp -> pat -> exp -> typ -> exp
val switch_variantE : exp -> (id * pat * exp) list -> typ -> exp
val switch_textE : exp -> (string * exp) list -> (pat * exp) -> typ -> exp
val tupE : exp list -> exp
val breakE: id -> exp -> exp
val retE: exp -> exp
val immuteE: exp -> exp
val assignE : var -> exp -> exp
val assignVarE : id -> exp -> exp
val labelE : id -> typ -> exp -> exp
val loopE : exp -> exp
val forE : pat -> exp -> exp -> exp
val loopWhileE : exp -> exp -> exp
val whileE : exp -> exp -> exp

val falseE : unit -> exp
val trueE : unit -> exp
val notE : exp -> exp
val andE : exp -> exp -> exp
val orE : exp -> exp -> exp
val impliesE : exp -> exp -> exp
val oldE : exp -> exp
val conjE : exp list -> exp

val declare_idE : id -> typ -> exp -> exp
val define_idE : id -> mut -> exp -> exp
val newObjE : obj_sort -> Ir.field list -> typ -> exp

val unreachableE : unit -> exp

(* Declarations *)

val letP : pat -> exp -> dec
val letD : var -> exp -> dec
val varD : var -> exp -> dec
val refD : var -> lexp -> dec
val expD : exp -> dec
val funcD : var -> var -> exp -> dec
val nary_funcD : var -> var list -> exp -> dec

val let_no_shadow : var -> exp -> dec list -> dec list

(* Continuations with explicit answer type *)

val contT : typ -> typ -> typ
val err_contT : typ -> typ
val bail_contT : typ
val clean_contT : typ
val answerT : typ -> typ (* answer type of a continuation type *)

(* Sequence expressions *)

val seqE : exp list -> exp

(* Lambdas *)

val (-->) : var -> exp -> exp
val (-->*) : var list -> exp -> exp (* n-ary local *)
val forall : typ_bind list -> exp -> exp (* generalization *)
val named : string -> exp -> exp (* renaming a function *)
val (-*-) : exp -> exp -> exp (* application *)

(* Objects *)

val objE : obj_sort -> (lab * con) list -> (lab * exp) list -> exp

val objectE : obj_sort -> (lab * exp) list -> field list -> exp

(* Records *)

val recordE : (lab * exp) list -> exp

(* IC ic0.call_perform status checking *)

val check_call_perform_status : exp -> (exp -> exp) -> exp
