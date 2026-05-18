(* Default builder for [Macro_registry.expander] records.

   Given a [prim switch (DISCR_EXP) { ARMS }] body, produces an
   expander that:

   - [scrut_of u] returns [DISCR_EXP] with the prim-type's first
     value-parameter substituted by [u].  Lives here (not in
     [mo_def]) so we can use [Traversals.over_exp].

   - [cases_of legs] pairs [ARMS] with the user's [legs] positionally,
     reifies each arm's index pattern as an [IntLit] [LitP] (or
     wildcard / guarded variant), and wraps each leg body in
     [RefineE] with the arm's refinement clause.

   Pre-condition: caller has validated [arms]/[legs] match (matching
   lengths, well-formed refinements). *)

open Mo_def
open Mo_types
open Mo_values
open Source
open Syntax

let subst_var (name : string) (rep : exp) : exp -> exp =
  Traversals.over_exp (fun e ->
    match e.it with
    | VarE x when x.it = name -> rep
    | _ -> e)

let build (discr : exp) (val_param_name : string)
          (arms : prim_switch_arm list)
        : Macro_registry.expander =
  let scrut_of (user_scrut : exp) : exp =
    subst_var val_param_name user_scrut discr
  in
  let lit_pat (idx_pat : prim_idx_pat) (at : region) : pat =
    match idx_pat with
    | IdxLitP n ->
      { it = LitP (ref (IntLit (Numerics.Int.of_int n)));
        at; note = Type.Pre }
    | IdxWildP -> { it = WildP; at; note = Type.Pre }
    | IdxGuardP _ ->
      (* TODO slice 7: emit a guarded leg via [WhenP]. *)
      { it = WildP; at; note = Type.Pre }
  in
  let cases_of (legs : case list) : case list =
    if List.length arms <> List.length legs then
      raise (Invalid_argument
        (Printf.sprintf "expander_builder: arms=%d legs=%d"
           (List.length arms) (List.length legs)));
    List.map2 (fun (arm : prim_switch_arm) (leg : case) ->
      let body = leg.it.exp in
      let refined =
        { it = RefineE (arm.it.refinement, body);
          at = body.at;
          note = empty_typ_note }
      in
      let new_pat = lit_pat arm.it.pat leg.at in
      annotate false { pat = new_pat; exp = refined } leg.at)
    arms legs
  in
  Macro_registry.{ scrut_of; cases_of }
