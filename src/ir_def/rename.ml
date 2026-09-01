open Ir

(* Identifiers and labels reside in different namespaces
   so we need to distinguish them when renaming to avoid capture  *)

module Binder = struct
  type t = Id of id | Lab of id
  let compare t1 t2 =
    match t1, t2 with
    | (Id id1, Id id2) -> String.compare id1 id2
    | (Lab lab1, Lab lab2) -> String.compare lab1 lab2
    | (Id _, Lab _) -> -1
    | (Lab _, Id _) -> 1
end

module Renaming = Map.Make(Binder)


(* One traversal for each syntactic category, named by that category *)

let fresh_id id = Construct.fresh_id id ()

let id rho i =
  match Renaming.find_opt (Binder.Id i) rho with
  | Some i1 -> i1
  | None -> i

let lab rho l =
  match Renaming.find_opt (Binder.Lab l) rho with
  | Some l1 -> l1
  | None -> l

let id_bind rho i =
  let i' = fresh_id i in
  (i', Renaming.add (Binder.Id i) i' rho)

let rec ids_bind rho = function
  | [] -> rho
  | i::is' ->
    let (i', rho') = id_bind rho i in
    ids_bind rho' is'

let lab_bind rho i =
  let i' = fresh_id i in
  (i', Renaming.add (Binder.Lab i) i' rho)

let arg_bind rho a =
  let i', rho' = id_bind rho a.it in
  ({a with it = i'}, rho')

let rec prim rho p =
  Ir.map_prim Fun.id (lab rho) p (* rename BreakPrim id etc *)

and exp rho e  =  {e with it = exp' rho e.it}
and exp' rho = function
  | VarE (m, i)         -> VarE (m, id rho i)
  | LitE _ as e         -> e
  | PrimE (p, es)       -> PrimE (prim rho p, List.map (exp rho) es)
  | ActorE (ds, fs, { meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, t) ->
    let ds', rho' = decs rho ds in
    ActorE
      (ds',
       fields rho' fs,
       {meta;
        preupgrade = exp rho' preupgrade;
        postupgrade = exp rho' postupgrade;
        heartbeat = exp rho' heartbeat;
        timer = exp rho' timer;
        inspect = exp rho' inspect;
        low_memory = exp rho' low_memory;
        stable_type = stable_type;
        stable_record = exp rho' stable_record;
       },
       t)

  | AssignE (e1, e2)    -> AssignE (lexp rho e1, exp rho e2)
  | BlockE (ds, e1)     -> let ds', rho' = decs rho ds
                           in BlockE (ds', exp rho' e1)
  | IfE (e1, e2, e3)    -> IfE (exp rho e1, exp rho e2, exp rho e3)
  | SwitchE (e, cs)     -> SwitchE (exp rho e, cases rho cs)
  | LoopE e1            -> LoopE (exp rho e1)
  | LabelE (i, t, e)    -> let i', rho' = lab_bind rho i in
                           LabelE(i', t, exp rho' e)
  | AsyncE (s, tb, e, t) -> AsyncE (s, tb, exp rho e, t)
  | DeclareE (i, t, e)  -> let i',rho' = id_bind rho i in
                           DeclareE (i', t, exp rho' e)
  | DefineE (i, m, e)   -> DefineE (id rho i, m, exp rho e)
  | FuncE (x, s, c, tp, p, ts, e) ->
     let p', rho' = args rho p in
     let e' = exp rho' e in
     FuncE (x, s, c, tp, p', ts, e')
  | NewObjE (s, fs, t)  -> NewObjE (s, fields rho fs, t)
  | TryE (e, cs, cl)    -> TryE (exp rho e, cases rho cs, Option.map (fun (v, t) -> id rho v, t) cl)
  | SelfCallE (ts, e1, e2, e3, e4) ->
     SelfCallE (ts, exp rho e1, exp rho e2, exp rho e3, exp rho e4)

and lexp rho le = {le with it = lexp' rho le.it}
and lexp' rho = function
  | VarLE i  -> VarLE (id rho i)
  | DotLE (e, i) -> DotLE (exp rho e, i)
  | IdxLE (e1, e2) -> IdxLE (exp rho e1, exp rho e2)

and exps rho es  = List.map (exp rho) es

and fields rho fs =
  List.map (fun f -> { f with it = { f.it with var = id rho f.it.var } }) fs

and args rho as_ =
  match as_ with
  | [] -> ([],rho)
  | a::as_ ->
     let (a', rho') = arg_bind rho a in
     let (as_', rho'') = args rho' as_ in
     (a'::as_', rho'')

and pat rho p =
  let p', rho = pat' rho p.it in
  {p with it = p'}, rho

and pat' rho = function
  | WildP as p    -> (p, rho)
  | VarP i ->
    let i, rho' = id_bind rho i in
    (VarP i, rho')
  | TupP ps ->
    let (ps, rho') = pats rho ps in
    (TupP ps, rho')
  | ObjP pfs ->
    let (pats, rho') = pats rho (pats_of_obj_pat pfs) in
    (ObjP (replace_obj_pat pfs pats), rho')
  | LitP _ as p ->
    (p, rho)
  | OptP p ->
    let (p', rho') = pat rho p in
    (OptP p', rho')
  | TagP (i, p) ->
    let (p', rho') = pat rho p in
    (TagP (i, p'), rho')
  | AltP (p1, p2) ->
    let is1 = Freevars.M.keys (Freevars.pat p1) in
    assert begin
      let is2 = Freevars.M.keys (Freevars.pat p1) in
      List.compare String.compare is1 is2 = 0
    end;
    let rho' = ids_bind rho is1 in
    (AltP (pat_subst rho' p1, pat_subst rho' p2), rho')
  | AndP (p1, p2) ->
    let p1', rho'  = pat rho p1 in
    let p2', rho'' = pat rho' p2 in
    (AndP (p1', p2'), rho'')

and pats rho ps  =
  match ps with
  | [] -> ([], rho)
  | p::ps ->
    let (p', rho') = pat rho p in
    let (ps', rho'') = pats rho' ps in
    (p'::ps', rho'')

and pat_subst rho p =
    let p'  = pat_subst' rho p.it in
    {p with it = p'}

and pat_subst' rho = function
  | WildP as p -> p
  | VarP i ->
    VarP (id rho i)
  | TupP ps ->
    TupP (pats_subst rho ps)
  | ObjP pfs ->
    let pats = pats_subst rho (pats_of_obj_pat pfs) in
    ObjP (replace_obj_pat pfs pats)
  | LitP _ as p -> p
  | OptP p ->
    OptP (pat_subst rho p)
  | TagP (i, p) ->
    TagP (i, pat_subst rho p)
  | AltP (p1, p2) ->
    AltP (pat_subst rho p1, pat_subst rho p2)
  | AndP (p1, p2) ->
    AndP (pat_subst rho p1, pat_subst rho p2)

and pats_subst rho ps =
  List.map (pat_subst rho) ps

and case rho (c : case) =
  {c with it = case' rho c.it}
and case' rho { pat = p; exp = e} =
  let (p', rho') = pat rho p in
  let e' = exp rho' e in
  {pat=p'; exp=e'}

and cases rho cs = List.map (case rho) cs

and dec rho d =
  let (mk_d, rho') = dec' rho d.it in
  ({d with it = mk_d}, rho')

and dec' rho = function
  | LetD (p, e) ->
     let p', rho = pat rho p in
     (fun rho' -> LetD (p',exp rho' e)),
     rho
  | VarD (i, t, e) ->
     let i', rho = id_bind rho i in
     (fun rho' -> VarD (i', t, exp rho' e)),
     rho
  | RefD (i, t, le) ->
     let i', rho = id_bind rho i in
     (fun rho' -> RefD (i', t, lexp rho' le)),
     rho

and decs rho ds =
  let rec decs_aux rho ds =
    match ds with
    | [] -> ([], rho)
    | d::ds ->
       let (mk_d, rho') = dec rho d in
       let (mk_ds, rho'') = decs_aux rho' ds in
       (mk_d::mk_ds, rho'')
  in
  let mk_ds, rho' = decs_aux rho ds in
  let ds' = List.map (fun mk_d -> { mk_d with it = mk_d.it rho' } ) mk_ds in
  (ds', rho')

let comp_unit rho cu = match cu with
  | ProgU ds ->
    let ds', _ = decs rho ds in
    ProgU ds'
  | LibU (ds, e) ->
    let ds', rho' = decs rho ds
    in LibU (ds', exp rho' e)
  | ActorU (as_opt, ds, fs, { meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type }, t) ->
    let as_opt', rho' = match as_opt with
      | None -> None, rho
      | Some as_ ->
        let as_', rho' = args rho as_ in
        Some as_', rho'
    in
    let ds', rho'' = decs rho' ds in
    ActorU (as_opt', ds', fields rho'' fs,
      { meta;
        preupgrade = exp rho'' preupgrade;
        postupgrade = exp rho'' postupgrade;
        heartbeat = exp rho'' heartbeat;
        timer = exp rho'' timer;
        inspect = exp rho'' inspect;
        low_memory = exp rho'' low_memory;
        stable_record = exp rho'' stable_record;
        stable_type = stable_type;
      }, t)
