open Mo_types
open Type
open Source

(* `bi_match scope_opt tbs subs ret_opt` returns
   a minimal instantiation `ts` such that
     * |`ts`| = |`tbs`|
     * `ts` satisfies the bounds in `tbs`
     * every `(t1,t2)` in `subs` satisfies `open_ ts t1 <: open_ ts t2`
     * any `Scope` parameter in `tbs` is instantiated with scope_opt, iff scope_opt <> None.
   or fails with exception `Failure msg` if
     * `tbs` contains open bounds mentioning parameters in `tbs` (a limitation); or
     * no such instantiation exists due to
       * a subtype violation; or
       * some parameter in `tbs` being under constrained (`ts` is ambiguous, see below); or
       * some parameter in `tbs` being over constrained (no `ts` exists); or
   For every pair (t1, t2) in `subs`,  one of `t1` or `t2` must be closed
   w.r.t `tbs` and the other may be "open" mentioning parameters in `tbs`
   by index (Var i, i < |`tbs`|).
   (This a pre-condition that avoids the need for full unification.)

   The ordering on instantiations `ts` is determined pointwise depending on the
   occurrence of that variable in `subs` and is:
   * sub-typing ( _ <: _) on variables that occur strictly positively in `subs`
   * super-typing ( _ :> _) on variables that occur strictly negatively in `subs`
   * equivalence ( _ = _ ) on variables that occur both positively and negatively in `subs`
   * trivial relation {(Non,Non)} on variables that don't occur at all in subs

   The optional formal return type, `ret_typ`, used when synthesizing an application,
   and mentioning parameters in `tbs`, is used to determine whether the inferred instantiation is
   ambiguous, guided by the polarities of type parameters from `tbs` in `ret_opt`.

   The inference algorithm infers the lower and upper bounds of each instantiation given the constraints
   in `subs` (and initial bounds on tbs).

   Given a parameter, `T`, if its inferred upper and lower bound coincide, the instantiation is the lower (=upper) type.
   Otherwise:
   * if the lower bound is (a proper) supertype of the upper bound, reject (no sastifying instantiation of `T` exists).
   * if the lower bound is (a proper) subtype of the upper bound and `T` occurs in `ret_opt` as:
     * positive (use the (principal) lower bound)
     * negative (use the (principal) upper bound)
     * neutral (use the lower bound)
     * invariant : reject the instantiation of `T` as under-constrained - no principal solution exists.

   (modulo mixing my left foot with my right)
*)

type reason =
  { actual : typ; expected : typ; at : region }

exception Bimatch of {
  message : string;
  hint : string option;
  reason : reason option;
}

val name_ret_typ : typ -> typ

(** Opaque context type for bi-matching *)
type ctx

(** General parameter inference for a conjunction of subtype problems.

  Solving can be done in two rounds:
  - Call the solver with the current set of sub-type problems to get the partial solution
  - Use the result to substitute solved type parameters (might contain remaining type variables)
  - Call the [finalize] function with the remaining sub-type problems (make sure the solved type parameters are substituted!)
    to get the final solution and the substitution of the remaining type variables from the 1st round

  [bi_match_subs scope_opt tbs ret_opt subs deferred]:
  - [scope_opt] is the optional async scope
  - [tbs] is the list of type parameters to instantiate
  - [ret_opt] is the optional return type mentioning tbs determining polarities
  - [subs] is the list of sub-type problems mentioning tbs either on left or right, but never both sides
  - [must_solve] is the list of types that must be solved in the 1st round.
    If empty, all type variables in [tbs] will be solved in the 1st round.
    Otherwise, only the type variables that appear free in [must_solve] will be solved in the 1st round.

  Returns: the solution and the remaining context for variables to be solved in the 2nd round.

  Raises: [Bimatch] when the problems are not solvable.
 *)
val bi_match_subs :
  scope option ->
  bind list ->
  typ option ->
  (typ * typ * region) list ->
  must_solve: typ list ->
  typ list * ctx

(** [finalize ts ctx subs] returns the final solution and the substitution of the remaining type variables from the 1st round.
  - [ts] is the solution from the 1st round
  - [ctx] is the remaining context for variables to be solved in this 2nd round
  - [subs] is the 2nd round sub-type problems

  Returns: the final solution and the substitution of the remaining type variables from the 1st round.

  Raises: [Bimatch] when the problems are not solvable.
 *)
val finalize :
  typ list ->
  ctx ->
  (typ * typ * region) list ->
  typ list * typ ConEnv.t

(** Checks that all types are closed (no unresolved type variables) *)
val fail_when_types_are_not_closed : ctx -> typ list -> unit

(** Checks that the given type is closed (no unresolved type variables) *)
val is_closed : ctx -> typ -> bool

(** Indicates whether debug prints are enabled *)
val debug : bool
