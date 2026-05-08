open Ir_def
open Mo_types

open Ir
open Ir_effect
open Type
open Construct

(* Optimize (self) tail calls to jumps, avoiding stack overflow
   in a single linear pass *)

(*
This is simple tail call optimizer that replaces tail calls to the current function by jumps.
It can  easily be extended to non-self tail calls, once supported by wasm.

For each function `f` whose `body[...]` has at least one self tailcall to `f<Ts>(es)`, apply the transformation:
```
    func f<Ts>(pat) = body[f<Ts>(es)+]
    ~~>
    func f<Ts>(args) = {
       var temp = args;
       loop {
         label l {
           let pat = temp;
           return body[{temp := es;break l;}+]
        }
      }
    }
```


It's implemented by a recursive traversal that maintains an environment recording whether the current term is in tail position,
and what its enclosing function (if any) is.

The enclosing function is forgotten when shadowed by a local binding (we don't assume all variables are distinct) and when
entering a function, class or actor constructor.

On little gotcha for functional programmers: the argument `e` to an early `return e` is *always* in tail position,
regardless of `return e`s own tail position.

TODO: optimize for multiple arguments using multiple temps (not a tuple).

*)

(* TRMC = tail recursion modulo constructor.
   When the function shape admits the v0 transform (List<A> -> List<B>),
   `trmc_info` carries the synthesised worker's identity and the parent's
   static type so that an in-place spine rewrite can build the wrapper-side
   code and a separate body construction can build the worker. *)
type trmc_info = { trmc_called: bool ref;        (* set when at least one spine fires *)
                   worker_var: Construct.var;    (* (worker_id, worker_fn_typ) *)
                   parent_var: Construct.var;    (* (parent_id, (B, List<A>)) *)
                   parent_typ: Type.typ;         (* (B, List<A>) *)
                   result_typ: Type.typ;         (* List<B>, for the wrapper-tail cast *)
                 }

type func_info = { func: id;
                   typ_binds: typ_bind list;
                   temps: var list;
                   label: id;
                   tail_called: bool ref;
                   trmc: trmc_info option;
                 }

type env = { tail_pos:bool;          (* is the expression in tail position *)
             info: func_info option; (* the innermost enclosing func, if any *)
           }


let bind env i (info:func_info option) : env =
  match info with
  | Some _ ->
    { env with info = info; }
  | None ->
    match env.info with
    | Some { func; _} when i = func ->
      { env with info = None } (* remove shadowed func info *)
    | _ -> env (* preserve existing, non-shadowed info *)

let bind_arg env a info = bind env a.it info


let are_generic_insts (tbs : typ_bind list) insts =
  List.for_all2 (fun (tb : typ_bind) inst ->
      match inst with
      | Con(c2,[]) -> Cons.eq tb.it.con c2 (* conservative, but safe *)
      |  _ -> false
      ) tbs insts

(* v0 TRMC eligibility:
   - Single-result function whose result type promotes to `?(B, _)`
   - At least one argument; first argument's type becomes the bullet's slot type
   Returns trmc_info if eligible, else None. *)
let try_v0_trmc_setup id funexp tbs =
  try
    let (sort, ctrl, _binds, dom, cod) =
      match Type.promote (typ funexp) with
      | Type.Func (a, b, c, d, e) -> (a, b, c, d, e)
      | _ -> raise Exit in
    (* Open the wrapper's closed function type using the wrapper's cons,
       so the worker's empty-bind signature can reference the same outer
       binders directly (the worker is nested in the wrapper's scope). *)
    let outer_cs =
      List.map (fun (tb : typ_bind) -> Type.Con (tb.it.con, [])) tbs in
    let dom_open = List.map (Type.open_ outer_cs) dom in
    let cod_open = List.map (Type.open_ outer_cs) cod in
    let first_arg_typ = match dom_open with t :: _ -> t | _ -> raise Exit in
    let result_typ = match cod_open with [t] -> t | _ -> raise Exit in
    (* Extract B from result_typ = ?(B, _); follow type aliases. *)
    let opt_inner = Type.as_opt_sub result_typ in
    let tup = Type.as_tup_sub 2 opt_inner in
    let b_ty = match tup with t :: _ :: _ -> t | _ -> raise Exit in
    let parent_typ = Type.Tup [b_ty; first_arg_typ] in
    let worker_id = fresh_id (id ^ "'") () in
    let parent_var = fresh_var "parent" parent_typ in
    let worker_fn_typ =
      Type.Func (sort, ctrl, [], parent_typ :: List.tl dom_open, [Type.unit]) in
    let worker_var = Construct.var worker_id worker_fn_typ in
    Some { trmc_called = ref false;
           worker_var;
           parent_var;
           parent_typ;
           result_typ }
  with _ -> None

(* Tail-call modulo constructor: does [e] place a self-call to [func]
   inside a fresh heap-allocator spine? `OptPrim` is operationally the
   identity on heap-allocated payloads, so it transparently extends the
   spine over `TupPrim`. Detection only — no rewrite (yet). *)
let rec self_call_in_modulo_constructor func e =
  match e.it with
  | PrimE (CallPrim _, [{it = VarE (_, f1); _}; _]) -> f1 = func
  | PrimE (TupPrim, es) -> List.exists (self_call_in_modulo_constructor func) es
  | PrimE (OptPrim, [inner]) -> self_call_in_modulo_constructor func inner
  | _ -> false

(* Build the worker body from the original exp0 (un-rewritten).
   v0: requires the body to be `SwitchE _ cases`. Each case is rewritten:
   - Recognised spine `?(head, self<Ts>(t, f))` →
       let cell = (head, t); StorePrim (parent.1, ?cell); return_call worker(cell, f)
   - Otherwise → unitE () (assumes the bullet was pre-set correctly). *)
let build_worker_body trmc func_id orig_body =
  let bullet () = projE (varE trmc.parent_var) 1 in
  let rewrite_case (c : case) =
    let { pat; exp = arm_body } = c.it in
    let new_arm = match arm_body.it with
      | PrimE (OptPrim, [{it = PrimE (TupPrim, [head_e; recur]); _}])
        when (match recur.it with
              | PrimE (CallPrim _, [{it = VarE (_, f1); _}; _]) -> f1 = func_id
              | _ -> false) ->
        let recur_args = match recur.it with
          | PrimE (CallPrim _, [_; {it = PrimE (TupPrim, args); _}]) -> args
          | _ -> assert false in
        let t_arg = List.nth recur_args 0 in
        let f_arg = List.nth recur_args 1 in
        let cell = fresh_var "cell" trmc.parent_typ in
        let opt_cell = optE (varE cell) in
        let store = primE StorePrim [bullet (); opt_cell] in
        let tail_call =
          let ce = callE (varE trmc.worker_var) [] (tupE [varE cell; f_arg]) in
          { ce with it = match ce.it with
            | PrimE (CallPrim ts, args) -> PrimE (TailCallPrim ts, args)
            | _ -> assert false } in
        blockE
          [letD cell (tupE [head_e; t_arg])]
          (blockE [expD store] tail_call)
      | _ -> unitE ()
    in
    { c with it = { pat; exp = new_arm } }
  in
  match orig_body.it with
  | SwitchE (_, cases) ->
    let new_cases = List.map rewrite_case cases in
    { orig_body with
      it = SwitchE (bullet (), new_cases);
      note = { orig_body.note with Note.typ = Type.unit } }
  | _ -> failwith "TRMC: worker body shape unsupported (v0)"

let rec tailexp env e =
  {e with it = exp' env e}

and exp env e  : exp =
  {e with it = exp' {env with tail_pos = false}  e}

and assignEs vars exp : dec list =
  match vars, exp.it with
  | [v], _ -> [ expD (assignE v exp) ]
  | _, PrimE (TupPrim, es) when List.length es = List.length vars ->
       List.map expD (List.map2 assignE vars es)
  | _, _ ->
    let tup = fresh_var "tup" (typ exp) in
    letD tup exp ::
    List.mapi (fun i v -> expD (assignE v (projE (varE v) i))) vars

and exp' env e  : exp' = match e.it with
  | (VarE (_, _) | LitE _) as it -> it
  | AssignE (e1, e2)    -> AssignE (lexp env e1, exp env e2)
  | PrimE (CallPrim insts, [e1; e2])  ->
    begin match e1.it, env with
    | VarE (_, f1), { tail_pos = true;
                      info = Some { func; typ_binds; temps; label; tail_called; _ } }
         when not !Mo_config.Flags.experimental_tailcalls
           && f1 = func && are_generic_insts typ_binds insts  ->
      tail_called := true;
      (blockE (assignEs temps (exp env e2)) (breakE label (unitE ()))).it
    | _, { tail_pos = true; _ } when !Mo_config.Flags.experimental_tailcalls ->
      PrimE (TailCallPrim insts, [exp env e1; exp env e2])
    | _,_-> PrimE (CallPrim insts, [exp env e1; exp env e2])
    end
  | BlockE (ds, e)      -> BlockE (block env ds e)
  | IfE (e1, e2, e3)    -> IfE (exp env e1, tailexp env e2, tailexp env e3)
  | SwitchE (e, cs)     -> SwitchE (exp env e, cases env cs)
  | TryE (e, cs, vt)    -> TryE (exp env e, cases env cs, vt) (* TBR *)
  | LoopE e1            -> LoopE (exp env e1)
  | LabelE (i, t, e)    -> let env1 = bind env i None in
                           LabelE(i, t, exp env1 e)
  | PrimE (RetPrim, [e])-> PrimE (RetPrim, [tailexp { env with tail_pos = true } e])
  | AsyncE (s, tb, e, typ) -> AsyncE (s, tb, exp { tail_pos = true; info = None } e, typ)
  | DeclareE (i, t, e)  -> let env1 = bind env i None in
                           DeclareE (i, t, tailexp env1 e)
  | DefineE (i, m, e)   -> DefineE (i, m, exp env e)
  | FuncE (x, s, c, tbs, as_, ret_tys, exp0) ->
    (* Shared functions (post async-lowering: Shared+Replies / Shared+Returns)
       are wrapped at the wasm level by `message_start ; … ; message_cleanup`
       (state-machine transition + GC). The user body is therefore *not*
       in tail position from the wasm function's perspective: cleanup runs
       below it. Setting tail_pos = true here would let the producer arm
       emit TailCallPrim for the body's last call, which codegen lowers to
       `return_call` — bypassing the cleanup and leaving the lifecycle in
       InUpdate, so the next message traps. Only Local function bodies are
       genuinely in tail position. *)
    let body_in_tail_pos = s = Type.Local in
    let env1 = { tail_pos = body_in_tail_pos; info = None } in
    let env2 = args env1 as_ in
    let exp0' = if body_in_tail_pos then tailexp env2 exp0 else exp env2 exp0 in
    FuncE (x, s, c, tbs, as_, ret_tys, exp0')
  | SelfCallE (ts, exp1, exp2, exp3, exp4) ->
    let env1 = { tail_pos = true; info = None} in
    let exp1' = tailexp env1 exp1 in
    let exp2' = exp env exp2 in
    let exp3' = exp env exp3 in
    let exp4' = exp env exp4 in
    SelfCallE (ts, exp1', exp2', exp3', exp4')
  | ActorE (ds, fs, u, t) ->
    (* TODO: tco other upgrade fields? *)
    let u = { u with preupgrade = exp env u.preupgrade; postupgrade = exp env u.postupgrade; stable_record = exp env u.stable_record } in
    ActorE (snd (decs env ds), fs, u, t)
  | NewObjE (s,is,t)    -> NewObjE (s, is, t)
  (* TRMC v0 spine rewrite: `?(head, self<Ts>(t, f))` →
     `let root = (head, t); worker<Ts>(root, f); (root : ResultTy)` *)
  | PrimE (OptPrim, [{it = PrimE (TupPrim, [head_e; recur]); _}])
       when env.tail_pos
         && !Mo_config.Flags.experimental_tailcalls
         && (match env.info, recur.it with
             | Some { func; trmc = Some _; _ },
               PrimE (CallPrim _, [{it = VarE (_, f1); _}; _]) -> f1 = func
             | _ -> false) ->
    let trmc = match env.info with
      | Some { trmc = Some t; _ } -> t
      | _ -> assert false in
    trmc.trmc_called := true;
    let recur_args = match recur.it with
      | PrimE (CallPrim _, [_; {it = PrimE (TupPrim, args); _}]) -> args
      | _ -> assert false in
    let t_arg = exp env (List.nth recur_args 0) in
    let f_arg = exp env (List.nth recur_args 1) in
    let head_e' = exp env head_e in
    let root = fresh_var "root" trmc.parent_typ in
    let worker_call =
      callE (varE trmc.worker_var) [] (tupE [varE root; f_arg]) in
    let cast_root =
      primE (CastPrim (trmc.parent_typ, trmc.result_typ)) [varE root] in
    let block = blockE
      [letD root (tupE [head_e'; t_arg])]
      (blockE [expD worker_call] cast_root) in
    block.it
  | PrimE (p, es)       -> PrimE (p, List.map (exp env) es)

and lexp env le : lexp = {le with it = lexp' env le}

and lexp' env le : lexp' = match le.it with
  | VarLE i -> VarLE i
  | DotLE (e, sn)  -> DotLE (exp env e, sn)
  | IdxLE (e1, e2) -> IdxLE (exp env e1, exp env e2)

and args env as_ =
  List.fold_left (fun env a -> bind_arg env a None) env as_

and pat env p =
  let env = pat' env p.it in
  env

and pat' env = function
  | WildP
  | LitP _         -> env
  | VarP i         -> bind env i None
  | TupP ps        -> pats env ps
  | ObjP pfs       -> pats env (pats_of_obj_pat pfs)
  | OptP p
  | TagP (_, p)    -> pat env p
  | AltP (p1, _p2) -> pat env p1 (* both bind the same vars, ensured in check_pat *)

and pats env ps  =
  match ps with
  | [] -> env
  | p :: ps ->
    let env1 = pat env p in
    pats env1 ps

and case env (c : case) =
  { c with it = case' env c.it }
and case' env {pat=p;exp=e} =
  let env1 = pat env p in
  let e' = tailexp env1 e in
  { pat=p; exp=e' }


and cases env cs = List.map (case env) cs

and dec env d =
  let (mk_d,env1) = dec' env d in
  ({d with it = mk_d}, env1)

and dec' env d =
  match d.it with
  (* A local let bound function, this is what we are looking for *)
  (* TODO: Do we need to detect more? A tuple of functions? *)
  | LetD (({it = VarP id;_} as id_pat),
          ({it = FuncE (x, Local, c, tbs, as_, typT, exp0);_} as funexp)) ->
    let env = bind env id None in
    begin fun env1 ->
      let temps = fresh_vars "temp" (List.map (fun a -> Mut a.note) as_) in
      let label = fresh_id "tailcall" () in
      let tail_called = ref false in
      let trmc =
        if !Mo_config.Flags.experimental_tailcalls
        then try_v0_trmc_setup id funexp tbs
        else None in
      let env2 = { tail_pos = true;
                   info = Some { func = id;
                                 typ_binds = tbs;
                                 temps;
                                 label;
                                 tail_called;
                                 trmc } }
      in
      let env3 = args env2 as_ in (* shadow id if necessary *)
      let exp0' = tailexp env3 exp0 in
      let cs = List.map (fun (tb : typ_bind) -> Con (tb.it.con, [])) tbs in
      let trmc_fired = match trmc with
        | Some t -> !(t.trmc_called)
        | None -> false in
      if trmc_fired then
        let trmc = match trmc with Some t -> t | None -> assert false in
        let worker_body = build_worker_body trmc id exp0 in
        let worker_args = arg_of_var trmc.parent_var :: List.tl as_ in
        let worker_funexp =
          funcE (id_of_var trmc.worker_var) Local c [] worker_args [Type.unit] worker_body in
        let body_with_worker =
          blockE [letD trmc.worker_var worker_funexp] exp0' in
        LetD (id_pat, {funexp with it = FuncE (x, Local, c, tbs, as_, typT, body_with_worker)})
      else if !tail_called then
        let ids = match typ funexp with
          | Func( _, _, _, dom, _) ->
            fresh_vars "id" (List.map (fun t -> open_ cs t) dom)
          | _ -> assert false
        in
        let l_typ = Type.unit in
        let body =
          blockE (List.map2 (fun t i -> varD t (varE i)) temps ids) (
            loopE (
              labelE label l_typ (blockE
                (List.map2 (fun a t -> letD (var_of_arg a) (immuteE (varE t))) as_ temps)
                (retE exp0'))
            )
          )
        in
        LetD (id_pat, {funexp with it = FuncE (x, Local, c, tbs, List.map arg_of_var ids, typT, body)})
      else
        LetD (id_pat, {funexp with it = FuncE (x, Local, c, tbs, as_, typT, exp0')})
    end,
    env
  | LetD (p, e) ->
    let env = pat env p in
    (fun env1 -> LetD(p,exp env1 e)),
    env
  | VarD (i, t, e) ->
    let env = bind env i None in
    (fun env1 -> VarD(i, t, exp env1 e)),
    env
  | RefD (i, t, e) ->
    let env = bind env i None in
    (fun env1 -> RefD(i, t, lexp env1 e)),
    env

and decs env ds =
  let rec decs_aux env ds =
    match ds with
    | [] -> ([],env)
    | d::ds ->
      let (mk_d, env1) = dec env d in
      let (mk_ds, env2) = decs_aux env1 ds in
      (mk_d :: mk_ds,env2)
  in
  let mk_ds,env1 = decs_aux env ds in
  env1,
  List.map
    (fun mk_d ->
      let env2 = { env1 with tail_pos = false } in
      { mk_d with it = mk_d.it env2 })
    mk_ds

and block env ds exp =
  let (env1, ds') = decs env ds in
  ( ds', tailexp env1 exp)

and comp_unit env = function
  | LibU _ -> raise (Invalid_argument "cannot compile library")
  | ProgU ds -> ProgU (snd (decs env ds))
  | ActorU (as_opt, ds, fs, u, t)  ->
    (* TODO: tco other fields of u? *)
    let u = { u with
              preupgrade = exp env u.preupgrade;
              postupgrade = exp env u.postupgrade;
              stable_record = exp env u.stable_record;
            } in
    ActorU (as_opt, snd (decs env ds), fs, u, t)

and prog (cu, flavor) =
  let env = { tail_pos = false; info = None } in
  (comp_unit env cu, flavor)

(* validation *)

let transform = prog
