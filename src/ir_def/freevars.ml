open Ir
open Mo_types

(* We collect a few things along the way *)

(* We want to know:
   Is this variable used (potentially) captured?
   Is this variable used eagerly.

   capture = true; eager = false means it is _only_ used under lambdas
*)
type usage_info = { captured : bool; eager : bool }

let join u1 u2 = {
  captured = u1.captured || u2.captured;
  eager = u1.eager || u2.eager
}

module M = Env.Make(String)
module S = Set.Make(String)


(* A set of free variables *)
type f = usage_info M.t

(* Operations: Union and removal *)
let (++) : f -> f -> f = M.union (fun _ u1 u2 -> Some (join u1 u2))
let unions f xs = List.fold_left (++) M.empty (List.map f xs)
let (//) x y = M.remove y x

(* Operations: left-biased Map union *)
let (+-) = M.union (fun _ u1 _ -> Some u1)

(* A set of defined variables (with type information) *)
type td = Mo_types.Type.typ M.t

(* A combined set of free variables and defined variables,
   (in declarations) *)
type fd = f * td

let fd_of_defs m = (M.empty, m)

(* Operations: *)

(* This adds a set of free variables to a combined set *)
let (+++) ((f,d) : fd) x = ((++) f x, d)
(* This takes the union of two combined sets *)
let (++++) (f1, d1) (f2,d2) =
  (++) f1 f2,
  M.union (fun _ t _ -> Some t) d1 d2 (* any type is fine *)
let union_binders f xs = List.fold_left (++++) (M.empty, M.empty) (List.map f xs)

let diff (m1 : 'a M.t) (m2 : 'b M.t) : 'a M.t =
  M.merge (fun k v1 -> function None -> v1 | Some _ -> None) m1 m2

let map_of_set x s = S.fold (fun v m -> M.add v x m) s M.empty
let set_of_map m = M.fold (fun v _ m -> S.add v m) m S.empty

(* The bound variables from the second argument scope over the first *)
let (///) (x : f) ((f,d) : fd) = f ++ diff x d

(* Usage tracking:

   We track which variables may be captured.
   Initially, variables are not captured.
   All variables under a lambda become captured.
*)
let under_lambda : f -> f = M.map (fun _ -> { captured = true; eager = false })

(* Projections *)
let captured_vars : f -> S.t =
  fun f -> set_of_map (M.filter (fun _ u -> u.captured) f)
let eager_vars : f -> S.t =
  fun f -> set_of_map (M.filter (fun _ u -> u.eager) f)

(* This closes a combined set over itself (recursion or mutual recursion) *)
let close (f,d) = diff f d

(* One traversal for each syntactic category, named by that category *)

let rec pat p : td = match p.it with
  | WildP | LitP _  -> M.empty
  | VarP i          -> M.singleton i p.note
  | TupP ps         -> pats ps
  | ObjP pfs        -> pats (pats_of_obj_pat pfs)
  | OptP p
  | TagP (_, p)     -> pat p
  | AltP (p1, p2)   -> pat p1 +- pat p2
  | AndP (p1, p2)   -> pat p1 +- pat p2

and pats ps : td = List.(fold_left (+-) M.empty (map pat ps))

let arg a : fd = (M.empty, M.singleton a.it a.note)

let args as_ : fd = union_binders arg as_

let id i = M.singleton i {captured = false; eager = true}

(* The mutable fields of an IR object behave a bit like a lambda, in that they capture mutable
boxes by reference. So set captured = true for them. *)
let fields fs = unions (fun f ->
  M.singleton f.it.var {captured = Type.is_mut f.note; eager = true}
) fs

let rec exp e : f = match e.it with
  | VarE (_, i)         -> id i
  | LitE l              -> M.empty
  | PrimE (_, es)       -> exps es
  | AssignE (e1, e2)    -> lexp e1 ++ exp e2
  | BlockE (ds, e1)     -> close (decs ds +++ exp e1)
  | IfE (e1, e2, e3)    -> exps [e1; e2; e3]
  | SwitchE (e, cs)     -> exp e ++ cases cs
  | LoopE e1            -> exp e1
  | LabelE (i, t, e)    -> exp e
  | AsyncE (_, _, e, _) -> exp e
  | DeclareE (i, t, e)  -> exp e  // i
  | DefineE (i, m, e)   -> id i ++ exp e
  | FuncE (x, s, c, tp, as_, t, e) -> under_lambda (exp e /// args as_)
  | ActorE (ds, fs, u, _)  -> actor ds fs u
  | NewObjE (_, fs, _)  -> fields fs
  | TryE (e, cs, cl)    -> exp e ++ cases cs ++ (match cl with Some (v, _) -> id v | _ -> M.empty)
  | SelfCallE (_, e1, e2, e3, e4) -> under_lambda (exp e1) ++ exps [e2; e3; e4]

and actor ds fs u = close (decs ds +++ fields fs +++ system u)

and system {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; _} =
  under_lambda (exp preupgrade) ++
  under_lambda (exp postupgrade) ++
  under_lambda (exp heartbeat) ++
  under_lambda (exp timer) ++
  under_lambda (exp inspect) ++
  under_lambda (exp low_memory) ++
  under_lambda (exp stable_record)

and exps es : f = unions exp es

and lexp le : f = match le.it with
  | VarLE i              -> id i
  | DotLE (e1, _)        -> exp e1
  | IdxLE (e1, e2)       -> exps [e1; e2]

and case (c : case) = exp c.it.exp /// fd_of_defs (pat c.it.pat)

and cases cs : f = unions case cs

and dec d = match d.it with
  | LetD (p, e) -> fd_of_defs (pat p) +++ exp e
  | VarD (i, t, e) -> fd_of_defs (M.singleton i t) +++ exp e
  | RefD (i, t, e) -> fd_of_defs (M.singleton i t) +++ lexp e

(* The variables captured by a function. May include the function itself! *)
and captured e =
  List.map fst (M.bindings (exp e))

and decs ps : fd = union_binders dec ps
