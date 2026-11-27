(* The facade pass: a `public actor* class` exports its messages through its container.

   It runs on an actor's fields BEFORE typing, so the typer sees ordinary declarations. A
   `public actor* class K` on a container becomes a PRIVATE class plus one shared method per
   exported method `m` of K, `K_m(id : Principal, params)`, which reconstitutes
   `actor*(null, id)` and dispatches locally with the message's caller. In K's body, each
   exported method `m` becomes two fields:

     public func @m(@caller : Principal, params) : async* R { let <ctx> = {caller = @caller}; body }
     public func m(params) : async R =
       (prim "local_actor_send")((prim "local_actor_container")().K_m, (prim "local_actor_id")(me), params)

   A `query` method's impl is synchronous and its facade a shared query. Nested public classes
   export through the container's flat registry; exported class names must be unique (M0280).
   Only typing exists for now: lowering refuses every actor* program (Pipeline, M0279). *)

open Mo_def
open Syntax
open Source
module T = Mo_types.Type

let diags : Diag.messages ref = ref []
let exported : string list ref = ref []

let err at text = diags := Diag.error_message at "M0280" "type" text :: !diags

(* `(x : T, ...)`, `(x : T)`, `x : T` or `()` -> the annotated parameters; else None. *)
let facade_params (pat : pat) : (id * typ) list option =
  let rec one p = match p.it with
    | AnnotP ({it = VarP x; _}, t) -> Some (x, t)
    | ParP inner -> one inner
    | _ -> None in
  let rec go p = match p.it with
    | TupP ps ->
      List.fold_right (fun p acc -> match one p, acc with
        | Some x, Some xs -> Some (x :: xs)
        | _ -> None) ps (Some [])
    | ParP inner -> go inner
    | _ -> (match one p with Some x -> Some [x] | None -> None) in
  go pat

let export_method cls m self (sort : T.shared_sort) (ctx_pat : pat) (tbs : typ_bind list)
    (pat : pat) (ret : typ) (fbody : exp) (at : region) : (dec * dec * dec_field, string) result =
  match facade_params pat with
  | None -> Error (Printf.sprintf "exported method %s.%s requires annotated parameters `(x : T, …)`" cls m)
  | Some params ->
  let is_query = (sort = T.Query) in
  let has_scope = List.exists (fun (b : typ_bind) -> b.it.sort.it = T.Scope) tbs in
  let system_inst = (not is_query) && has_scope in
  let id t = t @@ at in
  let var_e t = VarE (t @~ at) @? at in
  let path t = PathT (IdH (id t) @= at, []) @! at in
  let mk_pat p = p @! at in
  let annot x t = mk_pat (AnnotP (mk_pat (VarP (id x)), t)) in
  let inst sys : inst = {it = (if sys then Some (true, []) else None); at; note = []} in
  (* A single parameter/argument is written bare, as the parser writes it (a 1-tuple would
     give the facade a different Candid argument shape); `()` for none. *)
  let tup_pat = function [p] -> p | ps -> mk_pat (TupP ps) in
  let tup_exp = function [e] -> e | es -> TupE es @? at in
  let tup_typ = function [t] -> t | ts -> TupT (List.map (fun t -> (None, t)) ts) @! at in
  (* The parser's scope binders (parser.mly scope_bind / ensure_scope_bind / funcT): a function
     TYPE with an async result, or a shared one, gets the anonymous scope binder first. *)
  let scope_bind_named x : typ_bind = {var = T.scope_var x @@ at; sort = T.Scope @@ at; bound = PrimT "Any" @! at} @= at in
  let ensure_scope tbs = match tbs with
    | (tb : typ_bind) :: _ when tb.it.sort.it = T.Scope -> tbs
    | _ -> scope_bind_named "" :: tbs in
  let func_t (sort : func_sort) tbs t1 t2 = match sort.it, t2.it with
    | T.Local, AsyncT _ | T.Shared _, _ -> FuncT (sort, ensure_scope tbs, t1, t2) @! at
    | _ -> FuncT (sort, tbs, t1, t2) @! at in
  let local_sort : func_sort = T.Local @@ at in  (* in a function TYPE *)
  let local_sp : sort_pat = T.Local @@ at in      (* on a function EXPRESSION *)
  (* FRESH pattern and expression nodes per use: the typer writes each pattern's note once (infer_pat asserts
     it is still Pre), so one list shared by the impl, closure and facade crashed it. *)
  let param_pats () = List.map (fun (x, t) -> annot x.it t) params in
  let param_vars () = List.map (fun (x, _) -> var_e x.it) params in
  let param_typs = List.map snd params in
  let impl_name = "@" ^ m in
  let facade_name = cls ^ "_" ^ m in
  let const = Const @@ at in
  (* An `async`/`async*` TYPE as the parser writes it: the default scope variable. *)
  let any_scope = scopeT at in
  (* (1) the impl: the author's body under the actor pattern's context `{caller}`; a computation
     for an update, synchronous for a query (whose body holds no system capability). *)
  let ctx_decs = match ctx_pat.it with
    | WildP -> []
    | _ -> [LetD (ctx_pat, ObjE ([], [{mut = const; id = id "caller"; exp = var_e "@caller"} @@ at]) @? at, None) @? at] in
  let impl_block = BlockE (ctx_decs @ [ExpD fbody @? at]) @? at in
  (* As the parser gives a block-bodied `func ... : async* R { ... }`: the body wrapped in its
     own async* (Syntax.asyncE with the function's scope bind) -- that is what grants the
     body its await/send capability. A query impl is synchronous. *)
  let impl_scope : typ_bind = {var = T.scope_var impl_name @@ at; sort = T.Scope @@ at; bound = PrimT "Any" @! at} @= at in
  (* The typer wraps the body of a function returning `async*` (elab_func_body), so the
     impl's body is the block itself. *)
  let _ = impl_scope in
  let impl_body = impl_block in
  let impl_ret = if is_query then ret else AsyncT (T.Cmp, any_scope, ret) @! at in
  let impl_pat = tup_pat (annot "@caller" (path "Principal") :: param_pats ()) in
  let impl_binds = if is_query then List.filter (fun (b : typ_bind) -> b.it.sort.it <> T.Scope) tbs else tbs in
  let impl_fn = FuncE (impl_name, local_sp, impl_binds, impl_pat, Some impl_ret, func_note true, impl_body) @? at in
  let impl = LetD (mk_pat (VarP (id impl_name)), impl_fn, None) @? at in
  (* (2) the local closure: `m(params) = send(<container>.K_m, id(me), params)`. *)
  let actor_star_obj fields = ObjT (T.LocalActor @@ at, fields) @! at in
  let call f arg = CallE (None, f, inst false, (false, ref arg)) @? at in
  let self_id =
    call (AnnotE (PrimE "local_actor_id" @? at,
            func_t local_sort [] (actor_star_obj []) (path "Principal")) @? at)
      (var_e self) in
  (* NOT generic in a scope: a scope-generic callee needs a send capability that a `= e` body
     does not have (M0047). Its `async<$>` is the CLOSURE's own scope (below), so the send's
     future is the closure's result, as an actor's own shared method returns its call's. *)
  let send_typ =
    FuncT (local_sort, [], TupT (List.map (fun t -> (None, t)) (path "Any" :: path "Principal" :: param_typs)) @! at,
      AsyncT (T.Fut, any_scope, ret) @! at) @! at in
  let facade_sort : func_sort = T.Shared (if is_query then T.Query else T.Write) @@ at in
  let facade_typ =
    func_t facade_sort [] (tup_typ (path "Principal" :: param_typs)) (AsyncT (T.Fut, any_scope, ret) @! at) in
  let container_typ = ObjT (T.Actor @@ at, [ValF (id facade_name, facade_typ, const) @@ at]) @! at in
  let container_ref =
    call (AnnotE (PrimE "local_actor_container" @? at,
            func_t local_sort [] (TupT [] @! at) container_typ) @? at)
      (TupE [] @? at) in
  let send =
    call (AnnotE (PrimE "local_actor_send" @? at, send_typ) @? at)
      (TupE ((DotE (container_ref, id facade_name, ref None) @? at) :: self_id :: param_vars ()) @? at) in
  (* A function annotated `: async T` has its body wrapped in `async` by the typer
     (elab_func_body), so the closure awaits the send rather than returning its future.
     Lowering refuses actor* for now, so the extra hop is never run. *)
  let closure_fn = FuncE (m, local_sp, [scope_bind_named ""], tup_pat (param_pats ()), Some (AsyncT (T.Fut, any_scope, ret) @! at),
    func_note false, AwaitE (T.AwaitFut false, send) @? at) @? at in
  let closure = LetD (mk_pat (VarP (id m)), closure_fn, None) @? at in
  (* (3) the facade on the container: `K_m(id : Principal, params)`; the target is named
     STRUCTURALLY (a nested class's name is not in the container's scope). *)
  let facade_pat = tup_pat (annot "id" (path "Principal") :: param_pats ()) in
  let impl_typ = func_t local_sort impl_binds (tup_typ (path "Principal" :: param_typs)) impl_ret in
  let target_typ = actor_star_obj [ValF (id impl_name, impl_typ, const) @@ at] in
  (* moc infers a `let` right-hand side, and a reference has no type of its own (M0058): the
     annotation goes on the expression, as moc's `actor "url" : T` idiom does. *)
  let bind =
    LetD (mk_pat (VarP (id "target")),
      AnnotE (LocalActorRefE (TupE [LitE (ref NullLit) @? at; var_e "id"] @? at) @? at, target_typ) @? at, None) @? at in
  let args = tup_exp ((DotE (var_e "msg", id "caller", ref None) @? at) :: param_vars ()) in
  let target_call = CallE (None, DotE (var_e "target", id impl_name, ref None) @? at, inst system_inst, (false, ref args)) @? at in
  let result = if is_query then target_call else AwaitE (T.AwaitCmp, target_call) @? at in
  (* As the parser gives a block-bodied `shared ... : async R { ... }`: the anonymous scope
     bind, and the body wrapped in its async (Syntax.asyncE with the method's scope bind). *)
  let block = BlockE [bind; ExpD result @? at] @? at in
  let method_scope : typ_bind = {var = T.scope_var facade_name @@ at; sort = T.Scope @@ at; bound = PrimT "Any" @! at} @= at in
  (* The typer adds the method's `async` (elab_func_body); the body is the block. *)
  let _ = method_scope in
  let facade_body = block in
  let facade_fn =
    FuncE (facade_name, T.Shared ((if is_query then T.Query else T.Write), mk_pat (VarP (id "msg"))) @@ at,
      (ensure_scope []), facade_pat, Some (AsyncT (T.Fut, scopeT at, ret) @! at), func_note true, facade_body) @? at in
  let facade = {dec = LetD (mk_pat (VarP (id facade_name)), facade_fn, None) @? at;
                vis = Public None @@ at; stab = None} @@ at in
  Ok (impl, closure, facade)

let rec export_class (cls_at : region) (typ_id : typ_id) (self : id) (dfs : dec_field list)
    : dec_field list * dec_field list =
  let name = typ_id.it in
  (* A second export of a name gets no facades: its would duplicate the first's (M0051 on top
     of the M0280 that already says so). *)
  let duplicate = List.mem name !exported in
  if duplicate then
    err typ_id.at (Printf.sprintf "public actor* class %s is exported twice; exported class names must be unique in the container (their façades share one Candid interface)" name);
  exported := name :: !exported;
  let body = ref [] and facades = ref [] in
  List.iter (fun (mf : dec_field) ->
    match mf.it.vis.it, mf.it.dec.it with
    | Public _, ClassD (par, sp, os, tid, tbs, p, t, self', inner) when os.it = T.LocalActor ->
      let rw, fs = export_class mf.at tid self' inner in
      body := {mf with it = {mf.it with dec = {mf.it.dec with it = ClassD (par, sp, os, tid, tbs, p, t, self', rw)};
                                        vis = Private @@ mf.it.vis.at}} :: !body;
      facades := List.rev_append fs !facades
    | Public _, LetD ({it = VarP m; _},
        {it = FuncE (_, {it = T.Shared (sort, ctx_pat); _}, tbs, pat, Some {it = AsyncT (T.Fut, _, ret); _}, _, fbody); _}, None) ->
      (* The parser wrapped a block body in its async; the impl takes the body inside it. *)
      let fbody = match fbody.it with AsyncE (None, T.Fut, _, e) -> e | _ -> fbody in
      (match export_method name m.it self.it sort ctx_pat tbs pat ret fbody mf.at with
       | Ok (impl, closure, facade) ->
         body := {mf with it = {mf.it with dec = closure}} :: {mf with it = {mf.it with dec = impl}} :: !body;
         facades := facade :: !facades
       | Error text -> err mf.at text; body := mf :: !body)
    | _ -> body := mf :: !body) dfs;
  ignore cls_at;
  (List.rev !body, if duplicate then [] else List.rev !facades)

(* The fields of an actor (the container). *)
let facade_fields (dfs : dec_field list) : dec_field list =
  List.concat_map (fun (df : dec_field) ->
    match df.it.vis.it, df.it.dec.it with
    | Public _, ClassD (par, sp, os, tid, tbs, p, t, self, inner) when os.it = T.LocalActor ->
      let rw, fs = export_class df.at tid self inner in
      (* The class itself stays private: only the facades are the container's interface. *)
      {df with it = {df.it with dec = {df.it.dec with it = ClassD (par, sp, os, tid, tbs, p, t, self, rw)};
                                vis = Private @@ df.it.vis.at}} :: fs
    | _ -> [df]) dfs

let rec rewrite_exp (e : exp) : exp =
  match e.it with
  | ObjBlockE (eo, s, x, dfs) when s.it = T.Actor -> {e with it = ObjBlockE (eo, s, x, facade_fields dfs)}
  | AnnotE (e1, t) -> {e with it = AnnotE (rewrite_exp e1, t)}
  (* A top-level named actor parses as `let A = await async { actor A {...} }`. *)
  | AwaitE (s, e1) -> {e with it = AwaitE (s, rewrite_exp e1)}
  | AsyncE (par, s, tb, e1) -> {e with it = AsyncE (par, s, tb, rewrite_exp e1)}
  | _ -> e

let rewrite_dec (d : dec) : dec =
  match d.it with
  | ClassD (par, sp, s, tid, tbs, p, t, self, dfs) when s.it = T.Actor ->
    {d with it = ClassD (par, sp, s, tid, tbs, p, t, self, facade_fields dfs)}
  | ExpD e -> {d with it = ExpD (rewrite_exp e)}
  | LetD (p, e, f) -> {d with it = LetD (p, rewrite_exp e, f)}
  | _ -> d

(* The program with every actor's `public actor* class` fields rewritten, and the pass's
   diagnostics. Physically unchanged (and no diagnostics) without --experimental-local-actors. *)
let rewrite_prog (prog : prog) : prog * Diag.messages =
  if not !Mo_config.Flags.experimental_local_actors then (prog, []) else begin
    diags := []; exported := [];
    let prog' = {prog with it = List.map rewrite_dec prog.it} in
    (prog', List.rev !diags)
  end
