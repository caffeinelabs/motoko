(*
This module implements a check that rules out use-before define.
It is a compositional algorithm that returns, for each subexpression,
 * Which variables are used eagerly and
 * which are delayed
these sets are disjoint and their union makes up the set of free variables, so
the structure of this module is very similar to that of freevars.ml.

We keep it still separate, because we are doing much more interesting things
here in for blocks (function decs).
*)

open Mo_def
open Mo_types
open Source
open Syntax

(* We collect a few things along the way *)
type usage_info = Eager | Delayed

let join u1 u2 = match u1, u2 with
  | Eager, _ -> Eager
  | _, Eager -> Eager
  | Delayed, Delayed -> Delayed


module M = Env.Make(String)
module S = Set.Make(String)

let map_of_set x s = S.fold (fun v m -> M.add v x m) s M.empty
let set_unions = List.fold_left S.union S.empty

(* A set of free variables *)
type f = usage_info M.t

(* The analysis result of a recursive group, before tying the knot *)
type group = (Source.region * S.t * S.t * S.t) list

(* Operations: Union and removal *)
let (++) : f -> f -> f = M.union (fun _ u1 u2 -> Some (join u1 u2))
let unions f xs = List.fold_left (++) M.empty (List.map f xs)

(* A combined set of free variables and defined variables,
   e.g. in patterns and declaration *)
type defs = S.t
type fd = f * defs


(* Operations: *)

(* This adds a set of free variables to a combined set *)
let (+++) ((f, d) : fd) x = ((++) f x, d)
(* This takes the union of two combined sets *)
let (++++) (f1, d1) (f2, d2) = ((++) f1 f2, S.union d1 d2)
let union_binders f xs = List.fold_left (++++) (M.empty, S.empty) (List.map f xs)

let diff f d = M.filter (fun k _ -> not (S.mem k d)) f

(* The bound variables from the second argument scope over the first *)
let (///) (x : f) ((f, d) : fd) = f ++ diff x d

(* Usage tracking. We distinguish between eager and delayed variable use.
   Eager variables become delayed
   - inside lambda
   Delayed variables may stay delayed
   - when storing variables in data structures (tuples, objects, arrays)
   Delayed variables become eager
   - when occurs in an application
   - when a block uses some of its own variables eagerly
*)
let delayify : f -> f = M.map (fun _ -> Delayed)
let eagerify : f -> f = M.map (fun _ -> Eager)

let eager_vars : f -> S.t =
  fun f -> S.of_list (M.keys (M.filter (fun _ u -> u = Eager) f))
let delayed_vars : f -> S.t =
  fun f -> S.of_list (M.keys (M.filter (fun _ u -> u = Delayed) f))

(* One traversal for each syntactic category, named by that category *)

let rec exp msgs e : f = match e.it with
  (* Eager uses are either first-class uses of a variable: *)
  | HoleE (s, e) -> exp msgs (!e)
  | VarE i              -> M.singleton i.it Eager
  (* Or anything that is occurring in a call (as this may call a closure): *)
  | CallE (par_opt, e1, _ts, (_, e2)) -> eagerify (Option.to_list par_opt @ [e1; !e2] |> exps msgs)
  (* And break, return, throw can be thought of as calling a continuation: *)
  | BreakE (_, _, e)
  | RetE e
  | ThrowE e            -> eagerify (exp msgs e)
  (* Uses are delayed by function expressions *)
  | FuncE (_, sp, tp, p, t, _, e) ->
    delayify ((exp msgs e /// pat msgs p) /// shared_pat msgs sp)
  | ObjBlockE (eo, s, (self_id_opt, _), dfs) ->
    (* TBR: treatment of eo *)
    (match eo with
     | None -> M.empty
     | Some e1 -> eagerify (exp msgs e1)) ++
    group msgs (add_self self_id_opt s (dec_fields msgs dfs))
  (* The rest remaining cases just collect the uses of subexpressions: *)
  | LitE _
  | PrimE _ | ImportE _ | ImplicitLibE _ -> M.empty
  | ObjE (bases, efs)   -> exps msgs bases ++ exp_fields msgs efs
  | TupE es
  | ArrayE (_, es)
  | ToCandidE es        -> exps msgs es
  | BlockE ds           -> group msgs (decs msgs ds)
  | IfE (e1, e2, e3)    -> exps msgs [e1; e2; e3]
  | SwitchE (e, cs)
  | TryE (e, cs, None)  -> exp msgs e ++ cases msgs cs
  | TryE (e, cs, Some f)-> exps msgs [e; f] ++ cases msgs cs
  | LoopE (e1, None, _)    -> exp msgs e1
  | LoopE (e1, Some e2, _)
  | WhileE (e1, e2, _)
  | AssignE (e1, e2)
  | IdxE (e1, e2)
  | BinE (_, e1, _, e2)
  | RelE (_, e1, _, e2)
  | AndE (e1, e2)
  | OrE (e1, e2) -> exps msgs [e1; e2]
  | ForE (p, e1, e2, _)    -> exp msgs e1 ++ (exp msgs e2 /// pat msgs p)
  | AsyncE (Some par, _, _, e) -> exps msgs [par; e]
  | UnE (_, _, e)
  | ShowE (_, e)
  | FromCandidE e
  | DotE (e, _, _)
  | ProjE (e, _)
  | NotE e
  | LabelE (_, _, e)
  | DebugE e
  | AsyncE (None, _, _, e)
  | AwaitE (_, e)
  | AssertE (_, e)
  | AnnotE (e, _)
  | OptE e
  | DoOptE e
  | BangE e
  | TagE (_, e)
  | ActorUrlE e
  | IgnoreE e           -> exp msgs e

and exps msgs es : f = unions (exp msgs) es

and exp_fields msgs efs : f = unions (exp_field msgs) efs
and exp_field msgs ef : f = exp msgs ef.it.exp

and pat msgs p : fd = match p.it with
  | WildP         -> (M.empty, S.empty)
  | VarP i        -> (M.empty, S.singleton i.it)
  | TupP ps       -> pats msgs ps
  | ObjP pfs      -> pat_fields msgs pfs
  | AnnotP (p, _)
  | ParP p        -> pat msgs p
  | LitP l        -> (M.empty, S.empty)
  | SignP (uo, l) -> (M.empty, S.empty)
  | OptP p
  | TagP (_, p)   -> pat msgs p
  | AltP (p1, p2) -> pat msgs p1 ++++ pat msgs p2

and pats msgs ps : fd = union_binders (pat msgs) ps

and pat_fields msgs pfs =
  union_binders (fun pf ->
      match pf.it with
      | ValPF(_, p) -> pat msgs p
      | TypPF(_) -> (M.empty, S.empty)) pfs

and shared_pat msgs shared_pat =
  match shared_pat.it with
  | Type.Local ->
    (M.empty, S.empty)
  | Type.Shared (_, p1) ->
    pat msgs p1

and case msgs (c : case) = exp msgs c.it.exp /// pat msgs c.it.pat

and cases msgs cs : f = unions (case msgs) cs

and dec_fields msgs dfs =
  decs msgs (List.map (fun df -> df.it.dec) dfs)

and dec msgs d = match d.it with
  | ExpD e -> (exp msgs e, S.empty)
  | LetD (p, e, None) -> pat msgs p +++ exp msgs e
  | LetD (p, e, Some f) -> pat msgs p +++ exp msgs e +++ exp msgs f
  | VarD (i, e) -> (M.empty, S.singleton i.it) +++ exp msgs e
  | TypD (i, tp, t) -> (M.empty, S.empty)
  | ClassD (eo, csp, s, i, tp, p, t, i', dfs) ->
     ((M.empty, S.singleton i.it) +++
     (* TBR: treatment of eo *)
     (match eo with
      | None -> M.empty
      | Some e -> delayify (exp msgs e /// shared_pat msgs csp))
     ) +++
     delayify (
      group msgs (add_self (Some i')  s (dec_fields msgs dfs)) /// pat msgs p /// shared_pat msgs csp
    )
  | MixinD (p, ds) ->
     (group msgs (dec_fields msgs ds) /// pat msgs p, S.empty)
  | IncludeD (_, e, _) ->
     (exp msgs e, S.empty)

(* The self binding, if any, is treated as defined at the very beginning or end of the group,
   depending on sort and shadowing  *)
and add_self self_id_opt s group =
  match self_id_opt with
  | None -> group
  | Some i ->
    if List.exists (fun (at, defs, _, _) -> S.mem i.it defs) group
    then group (* shadowed, ignore *)
    else (* not shadowed, consider self ... *)
      let item = (i.at, S.singleton i.it, S.empty, S.empty) in
      match s.it with
      | Type.Actor -> item :: group (* ... defined early *)
      | _ -> group @ [item] (* ... defined late *)

and decs msgs decs : group =
  (* Annotate the declarations with the analysis results *)
  List.map (fun d ->
    let (f, defs) = dec msgs d in
    (d.at, defs, eager_vars f, delayed_vars f)
  ) decs

and group msgs (grp : group) : f =
  (* Create a map from declared variable to their definition point *)
  let defWhen = M.disjoint_unions (List.mapi (fun i (_, defs, _, _) -> map_of_set i defs) grp) in
  (* Calculate the relation R *)
  let r = NameRel.unions (List.map (fun (_, defs, _, delayed) -> NameRel.cross defs delayed) grp) in
  (* Check for errors *)
  List.iteri (fun i (at, _, eager, _) ->
    NameRel.iter (fun x y ->
      match M.find_opt y defWhen with
      | Some j ->
        (* At position i, we are evaluating something that requires y, which is
           defined after j *)
        if j < i
        then () (* all izz well *)
        else
          Diag.add_msg
            msgs
            (Diag.error_message
               at
               "M0016"
               "definedness"
               (Printf.sprintf "cannot use %s before %s has been defined" x y))
      | None ->
        (* External variable, ok for now *)
        ()
    ) (NameRel.restricted_rtcl eager r)
  ) grp;
  (* Now calculate the analysis result: Eager is everything that is eager, or
     used by eager things *)
  let e = set_unions (List.map (fun (_,_,eager,_) ->
    NameRel.range (NameRel.restricted_rtcl eager r)
  ) grp) in
  (* Everything else is lazy *)
  let d = S.diff (set_unions (List.map (fun (_,_,_,delayed) -> delayed) grp)) e in
  (* And remove whats defined here  *)
  M.disjoint_union (map_of_set Eager e) (map_of_set Delayed d) |>
    M.filter (fun v _ -> M.mem v defWhen = false)

let check_prog prog =
  Diag.with_message_store (fun msgs ->
    ignore (group msgs (decs msgs prog.it));
    Some ()
  )

let check_lib lib =
  Diag.with_message_store (fun msgs ->
    let (imp_ds, ds) = CompUnit.decs_of_lib lib in
    ignore (group msgs (decs msgs (imp_ds @ ds)));
    Some ()
  )
