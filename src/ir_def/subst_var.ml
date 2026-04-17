open Ir

(* One traversal for each syntactic category, named by that category *)
let id rho i =
  match Rename.Renaming.find_opt (Rename.Binder.Id i) rho with
  | Some i1 -> i1
  | None -> i

let id_bind rho i = Rename.Renaming.remove (Rename.Binder.Id i) rho
let ids_bind = List.fold_left id_bind

let arg_bind rho a = id_bind rho a.it

let rec exp rho e  =  {e with it = exp' rho e.it}
and exp' rho = function
  | VarE (m, i)         -> VarE (m, id rho i)
  | LitE _ as e         -> e
  | PrimE (p, es)       -> PrimE (p, List.map (exp rho) es)
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
  | LabelE (i, t, e)    ->  LabelE(i, t, exp rho e)
  | AsyncE (s, tb, e, t) -> AsyncE (s, tb, exp rho e, t)
  | DeclareE (i, t, e)  -> let rho' = id_bind rho i in
                           DeclareE (i, t, exp rho' e)
  | DefineE (i, m, e)   -> DefineE (id rho i, m, exp rho e)
  | FuncE (x, s, c, tp, p, ts, e) ->
     let rho' = args rho p in
     let e' = exp rho' e in
     FuncE (x, s, c, tp, p, ts, e')
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

and args = List.fold_left arg_bind

and pat rho p = pat' rho p.it

and pat' rho = function
  | WildP    -> rho
  | VarP i -> id_bind rho i
  | TupP ps -> pats rho ps
  | ObjP pfs -> pats rho (pats_of_obj_pat pfs)
  | LitP _ -> rho
  | OptP p -> pat rho p
  | TagP (i, p) -> pat rho p
  | AltP (p1, p2) ->
    let is1 = Freevars.M.keys (Freevars.pat p1) in
    assert begin
      let is2 = Freevars.M.keys (Freevars.pat p1) in
      List.compare String.compare is1 is2 = 0
    end;
    ids_bind rho is1

and pats rho ps  = List.fold_left pat rho ps

and case rho (c : case) =
  {c with it = case' rho c.it}
and case' rho { pat = p; exp = e} =
  let rho' = pat rho p in
  let e' = exp rho' e in
  {pat=p; exp=e'}

and cases rho cs = List.map (case rho) cs

and dec rho d =
  let (mk_d, rho') = dec' rho d.it in
  ({d with it = mk_d}, rho')

and dec' rho = function
  | LetD (p, e) ->
     let rho = pat rho p in
     (fun rho' -> LetD (p, exp rho' e)),
     rho
  | VarD (i, t, e) ->
     let rho = id_bind rho i in
     (fun rho' -> VarD (i, t, exp rho' e)),
     rho
  | RefD (i, t, le) ->
     let rho = id_bind rho i in
     (fun rho' -> RefD (i, t, lexp rho' le)),
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
    let rho' = match as_opt with
      | None -> rho
      | Some as_ -> args rho as_
    in
    let ds', rho'' = decs rho' ds in
    ActorU (as_opt, ds', fields rho'' fs,
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
