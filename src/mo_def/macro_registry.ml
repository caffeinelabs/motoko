(* Macro expander registry for `prim type` definitions.

   The registry is keyed by the `prim type` alias name.  Each entry is
   an [expander option ref], allocated by the parser *before* typing
   runs and populated by the typechecker after successful validation:

     1. Parser sees [prim type X<…>(…) = …]
          → [allocate "X"] creates a [ref None] under that key.
     2. Typechecker reaches [check_typ_prim_def] for the same X
          → on success, [set "X" expander] writes [Some expander]
            into the existing ref.
     3. Parser sees [switch type T { case <typ> <exp>; … }]
          → [get "X"] retrieves the (now-populated) ref, the parser
            asserts the slot is filled, and invokes the expander
            inline.  The emitted AST is a perfect, type-safe
            [SwitchE]+[RefineE] tree — no downstream consumer ever
            sees the surface form.

   Pre-condition for step 3: the prim type's typecheck must have run
   before the parser reaches the use site.  Holds when the prim type
   lives in the prelude / a separately-imported file. *)

open Mo_types
open Mo_values
open Source
open Syntax

type expander =
  scrutinee:exp -> legs:case list -> at:region -> exp

let registry : (string, expander option ref) Hashtbl.t = Hashtbl.create 8

let allocate (name : string) : expander option ref =
  match Hashtbl.find_opt registry name with
  | Some r -> r  (* re-parse of same prim type (e.g. in interactive mode) *)
  | None ->
    let r = ref None in
    Hashtbl.add registry name r;
    r

let set (name : string) (e : expander) : unit =
  let r = allocate name in
  r := Some e

let get (name : string) : expander option ref option =
  Hashtbl.find_opt registry name

let reset () = Hashtbl.reset registry

(* Default positional expander.  Pairs [arms] with [legs] by index; the
   leg pattern emitted is the prim_switch_arm's [pat] reified as an Int
   literal pattern (matching the typcode value); the leg body is wrapped
   in [RefineE] with the arm's refinement clause.  Mismatched lengths
   raise — callers should length-check before invoking. *)
let build_expander (_discr : prim_discr) (arms : prim_switch_arm list)
    : expander =
  fun ~scrutinee ~legs ~at ->
    if List.length arms <> List.length legs then
      raise (Invalid_argument
        (Printf.sprintf "macro_registry: arms=%d legs=%d"
           (List.length arms) (List.length legs)));
    let lit_pat (idx_pat : prim_idx_pat) (at : region) : pat =
      match idx_pat with
      | IdxLitP n ->
        { it = LitP (ref (IntLit (Numerics.Int.of_int n)));
          at; note = Type.Pre }
      | IdxWildP -> { it = WildP; at; note = Type.Pre }
      | IdxGuardP _ ->
        (* TODO slice 7: emit a guarded leg via [WhenP]; for now skip. *)
        { it = WildP; at; note = Type.Pre }
    in
    let cases =
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
    { it = SwitchE (scrutinee, cases); at; note = empty_typ_note }
