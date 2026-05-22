open Mo_types
open Ir_def
open Ir
open Lbool

(*
  This module identifies subexpressions that can be compiled statically. This
  means each subexpression must be a constant, immutable value for which the
  backend can create the memory representation statically.

  This module should stay in sync with the
   * the `compile_const_exp` function in `codegen/compile.ml`
   * the `Const.t` type in `codegen/compile.ml.
   * the checks in Check_ir.

  ## What is const?

  The high-level idea is

  * Variables can be const if their definition is const (beware, recursion!)
  * Blocks can be const if
    - all RHSs are const, and
    - no mutable variable are defined, and
    - pattern matching is side-effect free, i.e. irrefutable
  * Functions can be const if they do not require a closure.
    This is the case if every free variable is
    - const or
    - bound at the top level (`loc_known = true` below)
  * Literals can be const
  * Data structures can be const if they are immutable and all components are
    const
  * Projections can be const if they cannot fail (so no array index) and
    their argument is const

  I say “can be const” because the analysis does not have to be complete, just
  conservative.

  ## How does the analysis work?

  If we didn't have recursion, this would be simple: We'd pass down an environment
  mapping all free variables to whether they are constant or not (bool E.t), use this
  in the VarE case, and otherwise const-able expressions are constant when their
  subexpressions are.

  As always, recursion makes things harder. But not too much, thanks to a trick:
  we pass down a custom type called `Lbool.t`. It denotes a boolean value,
  just we do not know which one yet.  But we can still register implications on
  these values. Internally, these lazy_bool values keep track of their
  dependencies, and propagate additional information automatically. When one of them
  knows it is going to be surely false, then it updates the corresponding
  `note.const` field. See lbool.mli for more on that.

  This analysis relies on the fact that AST notes are mutable. So sharing AST
  nodes would be bad.  Check_ir checks for the absence of sharing.
*)

(* The environment *)

type lvl = TopLvl | NotTopLvl

module M = Env.Make(String)

type info = {
  loc_known : bool;
  const : Lbool.t;
}
type env = info M.t


let no_info = { loc_known = false; const = surely_false }
let arg lvl env a = M.add a.it { no_info with loc_known = lvl = TopLvl } env
let args lvl env as_ = List.fold_left (arg lvl) env as_

let rec pat env p = match p.it with
  | WildP
  | LitP _ -> env
  | VarP id -> M.add id no_info env
  | TupP pats -> List.fold_left pat env pats
  | ObjP pfs -> List.fold_left pat env (pats_of_obj_pat pfs)
  | AltP (pat1, _) | OptP pat1 | TagP (_, pat1) -> pat env pat1
  | AndP (pat1, pat2) -> pat (pat env pat1) pat2

let find v env = match M.find_opt v env with
  | None -> raise (Invalid_argument (Printf.sprintf "Unbound var: %s\n" v))
  | Some lb -> lb


(* Setting the notes *)

let set_const e b =
  if e.note.Note.const != b
  then e.note <- Note.{ e.note with const = b }

let set_lazy_const e lb =
  set_const e true;
  when_false lb (lazy (set_const e false))

(* Traversals *)

let rec exp lvl (env : env) e : Lbool.t =
  let lb =
    match e.it with
    | VarE (_, v) -> (find v env).const (*FIXME: use the mutability marker?*)
    | FuncE (x, s, c, tp, as_ , ts, body) ->
      exp_ NotTopLvl (args NotTopLvl env as_) body;
      begin match s, lvl with
      (* shared functions are not const for now *)
      | Type.Shared _, _ -> surely_false
      (* top-level functions can always be const (all free variables are top-level) *)
      | _, TopLvl -> surely_true
      | _, NotTopLvl ->
        let lb = maybe_false () in
        Freevars.M.iter (fun v _ ->
          let {loc_known; const} = find v env in
          if loc_known then () else (* static definitions are ok *)
          required_for const lb
        ) (Freevars.exp e);
        lb
      end
    | NewObjE (Type.(Object | Module), fs, t) when Type.is_immutable_obj t ->
      all (List.map (fun f -> (find f.it.var env).const) fs)
    | BlockE (ds, body) ->
      block lvl env (ds, body)
    | PrimE (TupPrim, es)
    | PrimE (ArrayPrim (Const, _), es) ->
      all (List.map (fun e -> exp lvl env e) es)
    | PrimE (DotPrim _, [e1] | ProjPrim _, [e1] | OptPrim, [e1] | TagPrim _, [e1]) ->
      exp lvl env e1
    | LitE _ ->
      surely_true

    (* All the following expressions cannot be const, but we still need to descend *)
    | PrimE (_, es) ->
      List.iter (exp_ lvl env) es;
      surely_false
    | DeclareE (id, _, e1) ->
      exp_ lvl (M.add id no_info env) e1;
      surely_false
    | LoopE e1 | AsyncE (_, _, e1, _) ->
      exp_ NotTopLvl env e1;
      surely_false
    | AssignE (_, e1) | LabelE (_, _, e1) | DefineE (_, _, e1) ->
      exp_ lvl env e1;
      surely_false
    | IfE (e1, e2, e3) ->
      exp_ lvl env e1;
      exp_ lvl env e2;
      exp_ lvl env e3;
      surely_false
    | SelfCallE (_, e1, e2, e3, e4) ->
      exp_ NotTopLvl env e1;
      exp_ lvl env e2;
      exp_ lvl env e3;
      exp_ lvl env e4;
      surely_false
    | SwitchE (e1, cs) | TryE (e1, cs, None) ->
      exp_ lvl env e1;
      List.iter (case_ lvl env) cs;
      surely_false
    | TryE (e1, cs, Some (v, t)) ->
      exp_ lvl env e1;
      List.iter (case_ lvl env) cs;
      exp_ lvl env Construct.(var v t |> varE);
      surely_false
    | NewObjE _ -> (* mutable objects *)
      surely_false
    | ActorE (ds, fs, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, _typ) ->
      (* this may well be “the” top-level actor, so don’t update lvl here *)
      let (env', _) = decs lvl env ds in
      exp_ lvl env' preupgrade;
      exp_ lvl env' postupgrade;
      exp_ lvl env' heartbeat;
      exp_ lvl env' timer;
      exp_ lvl env' inspect;
      exp_ lvl env' low_memory;
      exp_ lvl env' stable_record;
      surely_false
  in
  set_lazy_const e lb;
  lb

and exp_ lvl env e : unit = ignore (exp lvl env e)
and case_ lvl env c : unit =
  exp_ lvl (pat env c.it.pat) c.it.exp

and gather_dec lvl scope dec : env =
  let mk_info const = { loc_known = lvl = TopLvl; const } in
  let ok = match dec.it with
  | LetD (p, _) -> Ir_utils.is_irrefutable p
  | VarD _ | RefD _ -> false
  in
  M.fold (fun v _ scope ->
    if ok
    then M.add v (mk_info (maybe_false ())) scope
    else M.add v (mk_info surely_false) scope
  ) (snd (Freevars.dec dec)) scope (* TODO: implement gather_dec more directly *)

and gather_decs lvl ds : env =
  List.fold_left (gather_dec lvl) M.empty ds

and check_dec lvl env dec : Lbool.t = match dec.it with
  | LetD (p, e) when Ir_utils.is_irrefutable p ->
    let vs = snd (Freevars.dec dec) in (* TODO: implement gather_dec more directly *)
    let lb = exp lvl env e in
    M.iter (fun v _ -> required_for lb (M.find v env).const) vs;
    lb
  | VarD (_, _, e) | LetD (_, e) ->
    exp_ lvl env e;
    surely_false
  | RefD (_, _, _) ->
    surely_false

and check_decs lvl env ds : Lbool.t =
  all (List.map (check_dec lvl env) ds)

and decs lvl env ds : (env * Lbool.t) =
  let scope = gather_decs lvl ds in
  let env' = M.adjoin env scope in
  let could_be = check_decs lvl env' ds in
  (env', could_be)

and decs_ lvl env ds = ignore (decs lvl env ds)

and block lvl env (ds, body) =
  let (env', decs_const) = decs lvl env ds in
  let exp_const = exp lvl env' body in
  all [decs_const; exp_const]

and comp_unit = function
  | LibU _ -> raise (Invalid_argument "cannot compile library")
  | ProgU ds -> decs_ TopLvl M.empty ds
  | ActorU (as_opt, ds, fs, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, typ) ->
    let env = match as_opt with
      | None -> M.empty
      | Some as_ -> args TopLvl M.empty as_
    in
    let (env', _) = decs TopLvl env ds in
    exp_ TopLvl env' preupgrade;
    exp_ TopLvl env' postupgrade;
    exp_ TopLvl env' heartbeat;
    exp_ TopLvl env' timer;
    exp_ TopLvl env' inspect;
    exp_ TopLvl env' low_memory;
    exp_ TopLvl env' stable_record

let analyze ((cu, _flavor) : prog) =
  ignore (comp_unit cu)
