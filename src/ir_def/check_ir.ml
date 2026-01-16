open Mo_types
open Mo_values

open Source
module T = Type
module E = Ir_effect

(* TODO: make note immutable, perhaps just using type abstraction *)

(* TODO:
   dereferencing is still implicit in the IR (see immut_typ below);
   consider making it explicit as part of desugaring.
*)

(* TODO: check escape of free mutables via actors *)



(* Helpers *)

let (==>) p q = not p || q

let typ = E.typ

let immute_typ p =
  assert (not (T.is_mut (typ p)));
  typ p

(* Scope *)

type val_info = {
  typ : T.typ;
  (* see ir_passes/const.ml for the next two *)
  loc_known : bool;
  const : bool;
}
type val_env = val_info T.Env.t

type scope =
  { val_env : val_env;
  }

let empty_scope : scope =
  { val_env = T.Env.empty;
  }

(* Contexts (internal) *)

type lab_env = T.typ T.Env.t
type ret_env = T.typ option
(* the con_env tracks
   - which abstract types (type parameters) are in scope
   - which type aliases we are currently unfolding (to break recursion)
*)
type con_env = T.ConSet.t

type lvl = TopLvl | NotTopLvl

module Set = Set.Make(T.Ord)
module MapPair = Map.Make(T.OrdPair)

type env =
  { flavor : Ir.flavor;
    lvl  : lvl;
    vals : val_env;
    cons : con_env;
    labs : lab_env;
    rets : ret_env;
    async : T.con option;
    seen : con_env ref;
    check_run : int;
    check_typ_cache : Set.t ref;
    sub_cache : bool MapPair.t ref;
    lub_cache : T.typ MapPair.t ref;
  }

let last_run : int ref = ref 0

let initial_env flavor : env =
  let check_run = !last_run + 1 in
  last_run := check_run;
  { flavor;
    lvl = TopLvl;
    vals = T.Env.empty;
    cons = T.ConSet.empty;
    labs = T.Env.empty;
    rets = None;
    async = Async_cap.(match initial_cap() with
                       | NullCap | ErrorCap -> None
                       | QueryCap c | AwaitCap c | AsyncCap c | CompositeCap c | CompositeAwaitCap c | SystemCap c -> Some c);
    seen = ref T.ConSet.empty;
    check_run;
    check_typ_cache = ref Set.empty;
    sub_cache = ref MapPair.empty;
    lub_cache = ref MapPair.empty
  }

let sub env t1 t2 =
  if t1 == t2 then true else
  match MapPair.find_opt (t1, t2) !(env.sub_cache) with
  | Some b -> b
  | None ->
    let b = T.sub t1 t2 in
    env.sub_cache := MapPair.add (t1, t2) b !(env.sub_cache);
    b

let lub env t1 t2 =
  if t1 == t2 then t1 else
  match MapPair.find_opt (t1, t2) !(env.lub_cache) with
  | Some t -> t
  | None ->
    let t = T.lub t1 t2 in
    env.lub_cache := MapPair.add (t1, t2) t !(env.lub_cache);
    t

(* More error bookkeeping *)

exception CheckFailed of string

let type_error at : string -> Diag.message = Diag.error_message at "M0000" "IR type"

let error env at fmt =
    Printf.ksprintf (fun s -> raise (CheckFailed (Diag.string_of_message (type_error at s)))) fmt

let check env at p s =
  if not p then
    error env at "%s" s

let add_lab c x t = {c with labs = T.Env.add x t c.labs}

let add_typs c cs =
  { c with
    cons = List.fold_right (fun c -> T.ConSet.disjoint_add c) cs c.cons;
  }

let adjoin c scope =
  { c with
    vals = T.Env.adjoin c.vals scope.val_env;
  }

let adjoin_vals c ve = {c with vals = T.Env.adjoin c.vals ve}

let adjoin_cons c ce =
  { c with
    cons = T.ConSet.disjoint_union c.cons ce;
  }

let disjoint_union env at fmt env1 env2 =
  try T.Env.disjoint_union env1 env2
  with T.Env.Clash k -> error env at fmt k

(* Types *)

(* FIX ME: these error reporting functions are eager and will construct unnecessary type strings !*)
let check_sub env at t1 t2 =
  if not (sub env t1 t2) then
    error env at "subtype violation:\n  %s\n  %s\n"
      (T.string_of_typ_expand t1) (T.string_of_typ_expand t2)

let check_shared env at t =
  if not (T.shared t) then
    error env at "message argument is not sharable:\n  %s" (T.string_of_typ_expand t)

let check_concrete env at t =
  if not (T.concrete t) then
    error env at "message argument is not concrete:\n  %s" (T.string_of_typ_expand t)

let has_prim_eq t =
  (* Which types have primitive equality implemented in the backend? *)
  (* Keep in sync with Compile.compileq_eq *)
  let open T in
  match normalize t with
  | Prim Null -> false (* Ir_passes.Eq handles singleton types *)
  | Prim Error -> false (* should be desugared away *)
  | Prim _ -> true (* all other prims are fine *)
  | Non -> true
  (* references are handled in the back-end: *)
  | Obj (Actor, _, _) | Func (Shared _, _, _, _, _) -> true
  | _ -> false

let check_field_hashes env what at fields =
  Lib.List.iter_pairs
    (fun x y ->
      if Hash.hash x.T.lab = Hash.hash y.T.lab
      then error env at "field names %s and %s in %s type have colliding hashes"
        x.T.lab y.T.lab what;
    ) fields


let rec check_typ env typ : unit =
  match typ with
  | T.Pre ->
    error env no_region "illegal T.Pre type"
  | T.Var (s, i) ->
    error env no_region "free type variable %s, index %i" s  i
  | T.Con (c, typs) ->
    List.iter (check_typ env) typs;
    begin
      match Cons.kind c with
      | T.Def (tbs,_) ->
        check_con env c;
        check_typ_bounds env tbs typs no_region
      | T.Abs (tbs, _) ->
        if not (T.ConSet.mem c env.cons) then
          error env no_region "free type constructor %s " (T.string_of_typ typ);
        check_typ_bounds env tbs typs no_region
    end
  | T.Any -> ()
  | T.Non -> ()
  | T.Prim _ -> ()
  | T.Array typ ->
    check_mut_typ env typ
  | T.Tup typs ->
    List.iter (check_typ env) typs
  | T.Func (sort, control, binds, ts1, ts2) ->
    let cs, ce = check_typ_binds env binds in
    let env = adjoin_cons env ce in
    let ts = List.map (fun c -> T.Con (c, [])) cs in
    let ts1 = List.map (T.open_ ts) ts1 in
    let ts2 = List.map (T.open_ ts) ts2 in
    List.iter (check_typ env) ts1;
    List.iter (check_typ env) ts2;
    if T.is_shared_sort sort then begin
      List.iter (fun t -> check_shared env no_region t) ts1;
      match control with
      | T.Returns ->
        check env no_region (sort = T.Shared T.Write)
          "one-shot query function pointless";
        if not (ts2 = []) then
          error env no_region "one-shot function cannot have non-unit return types:\n  %s"
            (T.string_of_typ_expand (T.seq ts2));
      | T.Promises ->
        check env no_region (binds <> [])
          "promising function has no scope type argument";
        check env no_region env.flavor.Ir.has_async_typ
          "promising function in post-async flavor";
        if not (sort <> T.Local) then
          error env no_region "promising function cannot be local:\n  %s" (T.string_of_typ typ);
        if not (List.for_all T.shared ts2) then
          error env no_region "message result is not sharable:\n  %s" (T.string_of_typ typ)
      | T.Replies ->
        check env no_region (not env.flavor.Ir.has_async_typ)
          "replying function in pre-async flavor";
        if not (sort <> T.Local) then
          error env no_region"replying function cannot be local:\n  %s" (T.string_of_typ typ);
        if not (List.for_all T.shared ts2) then
          error env no_region "message result is not sharable:\n  %s" (T.string_of_typ typ)
      end else
     if not (control = T.Returns) then
       error env no_region "promising function cannot be local:\n  %s" (T.string_of_typ_expand typ);
  | T.Weak typ
  | T.Opt typ ->
    check_typ env typ
  | T.Async (s, typ1, typ2) ->
    check_typ env typ1;
    check_typ env typ2;
    check env no_region env.flavor.Ir.has_async_typ "async in non-async flavor";
    let t' = T.promote typ2 in
    check_shared env no_region t'
  | T.Obj (sort, fields, _) ->
    List.iter (check_typ_field env (Some sort)) fields;
    check_field_hashes env "object" no_region fields;
    (* fields strictly sorted (and) distinct *)
    if not (Lib.List.is_strictly_ordered T.compare_field fields) then
      error env no_region "object type's fields are not distinct and sorted %s" (T.string_of_typ typ)
  | T.Variant fields ->
    List.iter (check_typ_field env None) fields;
    check_field_hashes env "variant" no_region fields;
    if not (Lib.List.is_strictly_ordered T.compare_field fields) then
      error env no_region "variant type's fields are not distinct and sorted %s" (T.string_of_typ typ)
  | T.Mut typ ->
    error env no_region "unexpected T.Mut %s" (T.string_of_typ typ)
  | T.Named (_, typ1) ->
    check env no_region env.flavor.Ir.has_typ_field
     "named type field in non-typ_field flavor";
    check_typ env typ1


and check_mut_typ env = function
  | T.Mut t -> check_typ env t
  | t -> check_typ env t

and check_con env c =
  if T.ConSet.mem c !(env.seen) then ()
  else
  begin
    env.seen := T.ConSet.add c !(env.seen);
    let T.Abs (binds,typ) | T.Def (binds, typ) = Cons.kind c in
    check env no_region (not (T.is_mut typ)) "type constructor RHS is_mut";
    let cs, ce = check_typ_binds env binds in
    let ts = List.map (fun c -> T.Con (c, [])) cs in
    let env' = adjoin_cons env ce in
    check_typ env' (T.open_ ts typ)
  end

and check_typ_field env s tf : unit =
  match tf.T.typ, s with
  | T.Mut t, Some (T.Object | T.Memory) -> check_typ env t
  | t, Some T.Actor when not (T.is_shared_func t) ->
    error env no_region "actor field %s must have shared function type, found %s" tf.T.lab (T.string_of_typ t)
  | t, _ -> check_typ env t

and check_typ_binds_acyclic env cs ts  =
  let n = List.length cs in
  let ce = List.fold_right2 T.ConEnv.add cs ts T.ConEnv.empty in
  let chase c =
    let rec chase i ts c' =
      if i > n then
        error env no_region "type parameter %s has cyclic bounds %s"
          (T.string_of_con c)
          (String.concat " <: " (List.map T.string_of_typ ts)) (List.rev ts)
      else
        match T.ConEnv.find_opt c' ce with
        | None -> ()
        | Some t ->
          (match T.normalize t with
           | T.Con (c'', []) as t' ->
             chase (i+1) (t'::ts) c''
           | _ -> ())
    in chase 0 [] c
  in List.iter chase cs

and check_typ_binds env typ_binds : T.con list * con_env =
  let ts = Type.open_binds typ_binds in
  let cs = List.map (function T.Con (c, []) -> c | _ -> assert false) ts in
  let env' = add_typs env cs in
  let bds = List.map (fun typ_bind -> T.open_ ts typ_bind.T.bound) typ_binds in
  check_typ_binds_acyclic env' cs bds;
  List.iter (check_typ env') bds;
  cs, T.ConSet.of_list cs

and check_typ_bounds env (tbs : T.bind list) typs at : unit =
  let pars = List.length tbs in
  let args = List.length typs in
  if pars < args then
    error env at "too many type arguments";
  if pars > args then
    error env at "too few type arguments";
  List.iter2
    (fun tb typ ->
      check env at (sub env typ (T.open_ typs tb.T.bound))
        "type argument does not match parameter bound")
    tbs typs


and check_inst_bounds env tbs typs at =
  List.iter (check_typ env) typs;
  check_typ_bounds env tbs typs at

let check_typ env typ =
  if Set.mem typ !(env.check_typ_cache) then () else
  check_typ env typ;
  env.check_typ_cache := Set.add typ !(env.check_typ_cache);

(* Literals *)

open Ir

let type_lit env lit at : T.prim =
  match lit with
  | NullLit -> T.Null
  | BoolLit _ -> T.Bool
  | NatLit _ -> T.Nat
  | Nat8Lit _ -> T.Nat8
  | Nat16Lit _ -> T.Nat16
  | Nat32Lit _ -> T.Nat32
  | Nat64Lit _ -> T.Nat64
  | IntLit _ -> T.Int
  | Int8Lit _ -> T.Int8
  | Int16Lit _ -> T.Int16
  | Int32Lit _ -> T.Int32
  | Int64Lit _ -> T.Int64
  | FloatLit _ -> T.Float
  | CharLit _ -> T.Char
  | TextLit _ -> T.Text
  | BlobLit _ -> T.Blob


(* Expressions *)

let isAsyncE exp =
  match exp.it with
  | AsyncE _ (* pre await transformation *)
  | PrimE (CPSAsync _, [_]) (* post await transformation *)
    -> true
  | _ -> false

let store_typ t  =
  T.stable t &&
  match t with
  | T.Obj(T.Memory, fts, _) ->
    List.for_all (fun f -> T.is_opt f.T.typ) fts
  | _ -> false

let rec check_exp env (exp:Ir.exp) : unit =
  (* helpers *)
  let check p = check env exp.at p in
  let (<:) t1 t2 =
(*  try *)
      check_sub env exp.at t1 t2
(*  with e ->
     (Printf.eprintf "(in here):\n%s"
        (Wasm.Sexpr.to_string 80 (Arrange_ir.exp exp));
      raise e)
*)
  in
  (* check for aliasing *)
  if exp.note.Note.check_run = env.check_run
  then
    Printf.eprintf "IR has aliasing (or Check_ir visits nodes twice):\n%s"
      (Wasm.Sexpr.to_string 80 (Arrange_ir.exp exp));
  exp.note <- { exp.note with Note.check_run = env.check_run };
  (* check type annotation *)
  let t = E.typ exp in
  check_typ env t;
  (* check effect annotation *)
  check (E.infer_effect_exp exp <= E.eff exp)
    "inferred effect not a subtype of expected effect";
  (* check typing *)
  begin match exp.it with
  | VarE (m, id) ->
    let { typ; _ } =
      try T.Env.find id env.vals
      with Not_found -> error env exp.at "unbound variable %s" id
    in
    begin match m with
    | Const ->
       assert (not (T.is_mut typ));
       typ <: t
    | Var ->
       assert (T.is_mut typ);
       T.as_immut typ <: t
    end
  | LitE lit ->
    T.Prim (type_lit env lit exp.at) <: t
  | PrimE (p, es) ->
    List.iter (check_exp env) es;
    begin match p, es with
    | CallPrim insts, [exp1; exp2] ->
      begin match T.promote (typ exp1) with
        | T.Func (sort, control, tbs, arg_tys, ret_tys) ->
          check_inst_bounds env tbs insts exp.at;
          let t_arg = T.open_ insts (T.seq arg_tys) in
          let t_ret = T.codom control (fun () -> List.hd insts) (List.map (T.open_ insts) ret_tys) in
          if T.is_shared_sort sort then begin
            check_concrete env exp.at t_arg;
            check_concrete env exp.at t_ret;
          end;
          typ exp2 <: t_arg;
          t_ret <: t
        | T.Non -> () (* dead code, not much to check here *)
        | t1 -> error env exp1.at "expected function type, but expression produces type\n  %s"
             (T.string_of_typ_expand t1)
      end
    | UnPrim (ot, op), [exp1] ->
      check (Operator.has_unop op ot) "unary operator is not defined for operand type";
      typ exp1 <: ot;
      ot <: t
    | BinPrim (ot, op), [exp1; exp2] ->
      check (Operator.has_binop op ot) "binary operator is not defined for operand type";
      typ exp1 <: ot;
      typ exp2 <: ot;
      ot <: t
    | RelPrim (ot,  Operator.NeqOp), _ ->
      check false "negation operator should be desugared away in IR"
    | RelPrim (ot,  Operator.EqOp), [exp1; exp2] when (not env.flavor.has_poly_eq) ->
      check (has_prim_eq ot) "primitive equality is not defined for operand type";
      typ exp1 <: ot;
      typ exp2 <: ot;
      T.bool <: t
    | RelPrim (ot, op), [exp1; exp2] ->
      check (Operator.has_relop op ot) "relational operator is not defined for operand type";
      typ exp1 <: ot;
      typ exp2 <: ot;
      T.bool <: t
    | TupPrim, exps ->
      T.Tup (List.map typ exps) <: t
    | ProjPrim n, [exp1] ->
      let t1 = T.promote (immute_typ exp1) in
      let ts = try T.as_tup_sub n t1
               with Invalid_argument _ ->
                 error env exp1.at "expected tuple type, but expression produces type\n  %s"
                   (T.string_of_typ_expand t1) in
      let tn = try List.nth ts n with
               | Invalid_argument _ ->
                 error env exp.at "tuple projection %n is out of bounds for type\n  %s"
                   n (T.string_of_typ_expand t1) in
      tn <: t
    | OptPrim, [exp1] ->
      T.Opt (typ exp1) <: t
    | TagPrim i, [exp1] ->
      T.Variant [{T.lab = i; typ = typ exp1; src = T.empty_src}] <: t
    | ActorDotPrim n, [exp1]
    | DotPrim n, [exp1] ->
      begin
        let t1 = typ exp1 in
        let sort, tfs =
          try T.as_obj_sub [n] t1 with Invalid_argument _ ->
            error env exp1.at "expected object type, but expression produces type\n  %s"
              (T.string_of_typ_expand t1)
        in
        check (match p with
               | ActorDotPrim _ -> sort = T.Actor
               | DotPrim _ -> sort <> T.Actor
               | _ -> false) "sort mismatch";
        try T.as_immut (T.lookup_val_field n tfs) <: t with Invalid_argument _ ->
          error env exp1.at "field name %s does not exist in type\n  %s"
            n (T.string_of_typ_expand t1)
      end
    | ArrayPrim (mut, t0), exps ->
      List.iter (fun e -> typ e <: t0) exps;
      let t1 = T.Array (match mut with Const -> t0 | Var -> T.Mut t0) in
      t1 <: t
    | (IdxPrim | DerefArrayOffset), [exp1; exp2] ->
      let t1 = T.promote (typ exp1) in
      let t2 = try T.as_array_sub t1 with
               | Invalid_argument _ ->
                 error env exp1.at "expected array type, but expression produces type\n  %s"
                                         (T.string_of_typ_expand t1)
      in
      typ exp2 <: T.nat;
      T.as_immut t2 <: t
    | IdxBlobPrim, [exp1; exp2] ->
      typ exp1 <: T.blob;
      typ exp2 <: T.nat;
      T.(Prim Nat8) <: t
    | GetLastArrayOffset, [exp1] ->
      let t1 = T.promote (typ exp1) in
      ignore
        (try T.as_array_sub t1 with
         | Invalid_argument _ ->
           error env exp1.at "expected array type, but expression produces type\n  %s"
             (T.string_of_typ_expand t1));
      T.int <: t
    | NextArrayOffset, [exp1] ->
      typ exp1 <: T.nat;
      T.nat <: t
    | EqArrayOffset, [exp1; exp2] ->
      typ exp1 <: T.int;
      typ exp2 <: T.int;
      T.bool <: t
    | BreakPrim id, [exp1] ->
      begin
        match T.Env.find_opt id env.labs with
        | None -> error env exp.at "unbound label %s" id
        | Some t1 -> typ exp1 <: t1;
      end;
      T.Non <: t (* vacuously true *)
    | RetPrim, [exp1] ->
      begin
        match env.rets with
        | None -> error env exp.at "misplaced return"
        | Some t0 -> assert (t0 <> T.Pre); typ exp1 <: t0;
      end;
      T.Non <: t (* vacuously true *)
    | ThrowPrim, [exp1] ->
      check env.flavor.has_await "throw in non-await flavor";
      check (env.async <> None) "misplaced throw";
      typ exp1 <: T.throw;
      T.Non <: t (* vacuously true *)
    | AwaitPrim s, [exp1] ->
      check env.flavor.has_await "await in non-await flavor";
      let t0 = match env.async with
      | Some c -> T.Con(c, [])
      | None -> error env exp.at "misplaced await" in
      let t1 = T.promote (typ exp1) in
      let (t2, t3) = try T.as_async_sub (to_async_sort s) t0 t1
             with Invalid_argument _ ->
               error env exp1.at "expected async type, but expression has type\n  %s"
                 (T.string_of_typ_expand t1)
      in
      check (T.eq t0 t2) "ill-scoped async";
      t3 <: t
    | AssertPrim, [exp1] ->
      typ exp1 <: T.bool;
      T.unit <: t
    | ShowPrim ot, [exp1] ->
      check env.flavor.has_show "show expression in non-show flavor";
      check (Show.can_show ot) "show is not defined for operand type";
      typ exp1 <: ot;
      T.text <: t
    | SerializePrim ots, [exp1] ->
      check (T.shared (T.seq ots)) "Serialize is not defined for operand type";
      typ exp1 <: T.seq ots;
      T.blob <: t
    | DeserializePrim ots, [exp1] ->
      check (T.shared (T.seq ots)) "Deserialize is not defined for operand type";
      typ exp1 <: T.blob;
      T.seq ots <: t
    | DeserializeOptPrim ots, [exp1] ->
      check (T.shared (T.seq ots)) "DeserializeOpt is not defined for operand type";
      typ exp1 <: T.blob;
      T.Opt (T.seq ots) <: t
    | CPSAwait (s, cont_typ), [a; krb] ->
      let (_, t1) =
        try T.as_async_sub (to_async_sort s) T.Non (T.normalize (typ a))
        with _ -> error env exp.at "CPSAwait expect async arg, found %s" (T.string_of_typ (typ a))
      in
      (match cont_typ with
       | T.Func(T.Local, T.Returns, [], ts1, ts2) ->
         begin
           (match ts2 with
            | [] -> ()
            | _ -> error env exp.at "CPSAwait answer type error");
           typ krb <: T.(Tup Construct.[cont_typ; err_contT (seq ts2); bail_contT]);
           t1 <: T.seq ts1;
           T.seq ts2 <: t;
         end;
       | _ -> error env exp.at "CPSAwait bad cont");
      check (not (env.flavor.has_await)) "CPSAwait await flavor";
      check (env.flavor.has_async_typ) "CPSAwait in post-async flavor";
    | CPSAsync (s, t0), [exp] ->
      (match typ exp with
       | T.Func (T.Local, T.Returns, [tb],
                 T.[Func (Local, Returns, [], ts1, []);
                    Func (Local, Returns, [], [t_error], []);
                    Func (Local, Returns, [], [], [])],
                 []) ->
          T.catch <: t_error;
          T.Async(s, t0, T.open_ [t0] (T.seq ts1)) <: t
       | _ -> error env exp.at "CPSAsync unexpected typ");
      check (not (env.flavor.has_await)) "CPSAsync await flavor";
      check (env.flavor.has_async_typ) "CPSAsync in post-async flavor";
      check_typ env t;
    | ICArgDataPrim, [] ->
      T.blob <: t
    | ICReplyPrim ts, [exp1] ->
      check (not (env.flavor.has_async_typ)) "ICReplyPrim in async flavor";
      check (T.shared t) "ICReplyPrim is not defined for non-shared operand type";
      (* TODO: check against expected reply typ; note this may not be env.ret_tys. *)
      typ exp1 <: (T.seq ts);
      T.Non <: t
    | ICRejectPrim, [exp1] ->
      check (not (env.flavor.has_async_typ)) "ICRejectPrim in async flavor";
      typ exp1 <: T.text;
      T.Non <: t
    | ICCallerPrim, [] ->
      T.caller <: t
    | ICCallPrim, [exp1; exp2; k; r; c] ->
      let t1 = T.promote (typ exp1) in
      begin match t1 with
      | T.Func (sort, T.Replies, _ (*TBR*), arg_tys, ret_tys) ->
        let t_arg = T.seq arg_tys in
        typ exp2 <: t_arg;
        check_concrete env exp.at t_arg;
        typ k <: T.(Construct.contT (Tup ret_tys) unit);
        typ r <: T.(Construct.err_contT unit);
        typ c <: Construct.clean_contT;
      | T.Non -> () (* dead code, not much to check here *)
      | _ ->
         error env exp1.at "expected function type, but expression produces type\n  %s"
           (T.string_of_typ_expand t1)
      end
      (* TODO: T.unit <: t ? *)
    | ICCallRawPrim, [exp1; exp2; exp3; k; r; c] ->
      typ exp1 <: T.principal;
      typ exp2 <: T.text;
      typ exp3 <: T.blob;
      typ k <: T.(Construct.contT blob unit);
      typ r <: T.(Construct.err_contT unit);
      typ c <: Construct.clean_contT;
      T.unit <: t
    | ICMethodNamePrim, [] ->
      T.text <: t
    | ICReplyDeadlinePrim, [] ->
      T.nat64 <: t
    | ICStableRead t1, [] ->
      check_typ env t1;
      check (store_typ t1) "Invalid type argument to ICStableRead";
      t1 <: t
    | ICStableWrite t1, [] ->
      check_typ env t1;
      check (store_typ t1) "Invalid type argument to ICStableWrite";
      T.unit <: t
    | NumConvWrapPrim (p1, p2), [e] ->
      (* we should check if this conversion is supported *)
      typ e <: T.Prim p1;
      T.Prim p2 <: t
    | NumConvTrapPrim (p1, p2), [e] ->
      (* we should check if this conversion is supported *)
      typ e <: T.Prim p1;
      T.Prim p2 <: t
    | CastPrim (t1, t2), [e] ->
      typ e <: t1;
      t2 <: t
    | EncodeUtf8, [e] ->
      typ e <: T.text;
      T.blob <: t
    | DecodeUtf8, [e] ->
      typ e <: T.blob;
      T.(Opt text) <: t
    | BlobOfIcUrl, [e] ->
      typ e <: T.text;
      T.blob <: t
    | IcUrlOfBlob, [e] ->
      typ e <: T.blob;
      T.text <: t
    | ActorOfIdBlob actor_typ, [e] ->
      typ e <: T.blob;
      check_typ env actor_typ;
      begin match T.normalize actor_typ with
      | T.Obj (T.Actor, _, _) -> ()
      | _ -> error env exp.at "ActorOfIdBlob cast to actor object type, not\n   %s"
           (T.string_of_typ_expand actor_typ)
      end;
      actor_typ <: t;
    | SelfRef t1, [] ->
      (* We could additionally keep track of the type of the current actor in
         the environment and see if this lines up. *)
      t1 <: t;
    | SystemTimePrim, [] ->
      T.nat64 <: t;
    (* Cycles *)
    | (SystemCyclesBalancePrim | SystemCyclesAvailablePrim | SystemCyclesRefundedPrim), [] ->
      T.nat <: t
    | (SystemCyclesAcceptPrim | SystemCyclesBurnPrim), [e1] ->
      typ e1 <: T.nat;
      T.nat <: t
    | SystemCyclesAddPrim, [e1] ->
      typ e1 <: T.nat;
      T.unit <: t
    | SystemTimeoutSetPrim, [e1] ->
      typ e1 <: T.nat32;
      T.unit <: t
    (* Certified Data *)
    | SetCertifiedData, [e1] ->
      typ e1 <: T.blob;
      T.unit <: t
    | GetCertificate, [] ->
      T.Opt T.blob <: t
    | ICPerformGC, [] ->
      T.unit <: t
    | ICStableSize t1, [e1] ->
      typ e1 <: t1;
      T.nat64 <: t
    | OtherPrim _, _ -> ()
    | p, args ->
      error env exp.at "PrimE %s does not work with %d arguments"
        (Wasm.Sexpr.to_string 80 (Arrange_ir.prim p)) (List.length args);
    end
  | AssignE (lexp1, exp2) ->
    check_lexp env lexp1;
    check_exp env exp2;
    let t2 = try T.as_mut lexp1.note with
       Invalid_argument _ -> error env exp.at "expected mutable assignment target"
    in
    typ exp2 <: t2;
    T.unit <: t
  | BlockE (ds, exp1) ->
    let scope = gather_block_decs env ds in
    let env' = adjoin env scope in
    check_decs env' ds;
    check_exp env' exp1;
    typ exp1 <: t
  | IfE (exp1, exp2, exp3) ->
    check_exp env exp1;
    typ exp1 <: T.bool;
    check_exp env exp2;
    typ exp2 <: t;
    check_exp env exp3;
    typ exp3 <: t
  | SwitchE (exp1, cases) ->
    check_exp env exp1;
    let t1 = T.promote (typ exp1) in
(*    if not env.pre then
      if not (Coverage.check_cases env.cons cases t1) then
        warn env exp.at "the cases in this switch do not cover all possible values";
 *)
    check_cases env t1 t cases
  | TryE (exp1, cases, vt) ->
    check env.flavor.has_await "try in non-await flavor";
    check (env.async <> None) "misplaced try";
    check_exp env exp1;
    typ exp1 <: t;
    check_cases env T.catch t cases;
    Option.iter (fun (_, t) -> t <: Construct.bail_contT) vt
  | LoopE exp1 ->
    check_exp { env with lvl = NotTopLvl } exp1;
    typ exp1 <: T.unit;
    T.Non <: t (* vacuously true *)
  | LabelE (id, t0, exp1) ->
    assert (t0 <> T.Pre);
    check_typ env t0;
    check_exp (add_lab env id t0) exp1;
    typ exp1 <: t0;
    t0 <: t
  | AsyncE (s, tb, exp1, t0) ->
    check env.flavor.has_await "async expression in non-await flavor";
    check_typ env t0;
    let c, tb, ce = check_open_typ_bind env tb in
    let t1 = typ exp1 in
    let env' =
      {(adjoin_cons env ce)
       with labs = T.Env.empty; rets = Some t1; async = Some c; lvl = NotTopLvl} in
    check_exp env' exp1;
    let t1' = T.open_ [t0] (T.close [c] t1)  in
    t1' <: T.Any; (* vacuous *)
    (* check t1' shared when Fut? *)
    T.Async (s, t0, t1') <: t
  | DeclareE (id, t0, exp1) ->
    check_mut_typ env t0;
    let val_info = { typ = t0; loc_known = false; const = false } in
    let env' = adjoin_vals env (T.Env.singleton id val_info) in
    check_exp env' exp1;
    typ exp1 <: t
  | DefineE (id, mut, exp1) ->
    check_exp env exp1;
    begin
      match T.Env.find_opt id env.vals with
      | None -> error env exp.at "unbound variable %s" id
      | Some { typ = t0; const; loc_known } ->
        check (not const) "cannot use DefineE on const variable";
        match mut with
        | Const ->
          typ exp1 <: t0
        | Var ->
          let t0 = try T.as_mut t0 with
                   | Invalid_argument _ ->
                     error env exp.at "expected mutable %s" (T.string_of_typ t0)
          in
          typ exp1 <: t0
    end;
    T.unit <: t
  | FuncE (x, sort, control, typ_binds, args, ret_tys, exp) ->
    let cs, tbs, ce = check_open_typ_binds env typ_binds in
    let ts = List.map (fun c -> T.Con(c, [])) cs in
    let env' = adjoin_cons env ce in
    let ve = check_args env' args in
    List.iter (check_typ env') ret_tys;
    check ((T.is_shared_sort sort && control = T.Promises) ==> isAsyncE exp)
      "shared function with async type has non-async body";
    if T.is_shared_sort sort then List.iter (check_concrete env exp.at) ret_tys;
    let codom = T.codom control (fun () -> List.hd ts) ret_tys in
    let env'' =
      {env' with labs = T.Env.empty; rets = Some codom; async = None; lvl = NotTopLvl} in
    check_exp (adjoin_vals env'' ve) exp;
    check_sub env' exp.at (typ exp) codom;
    (* Now construct the function type and compare with the annotation *)
    let ts1 = List.map (fun a -> a.note) args in
    if T.is_shared_sort sort then List.iter (check_concrete env exp.at) ts1;
    let fun_ty = T.Func
      ( sort, control
      , tbs, List.map (T.close cs) ts1, List.map (T.close cs) ret_tys
      ) in
    fun_ty <: t
  | SelfCallE (ts, exp_f, exp_k, exp_r, exp_c) ->
    check (not env.flavor.Ir.has_async_typ) "SelfCallE in async flavor";
    List.iter (check_typ env) ts;
    check_exp { env with lvl = NotTopLvl } exp_f;
    check_exp env exp_k;
    check_exp env exp_r;
    check_exp env exp_c;
    typ exp_f <: T.unit;
    typ exp_k <: T.(Construct.contT (Tup ts) unit);
    typ exp_r <: T.(Construct.err_contT unit);
    typ exp_c <: Construct.clean_contT;
  | ActorE (ds, fs,
      { preupgrade; postupgrade; meta; heartbeat; timer; inspect; low_memory; stable_record; stable_type }, t0) ->
    (* TODO: check meta *)
    let env' = { env with async = None } in
    let scope1 = gather_block_decs env' ds in
    let env'' = adjoin env' scope1 in
    check_decs env'' ds;
    check_exp env'' preupgrade;
    check_exp env'' postupgrade;
    check_exp env'' heartbeat;
    check_exp env'' timer;
    check_exp env'' inspect;
    let async_cap = Some Async_cap.top_cap in
    check_exp { env'' with async = async_cap } low_memory;
    check_exp env'' stable_record;
    typ preupgrade <: T.unit;
    typ postupgrade <: T.unit;
    typ heartbeat <: T.unit;
    typ timer <: T.unit;
    typ inspect <: T.unit;
    typ low_memory <: T.unit;
    typ stable_record <: stable_type.post;
    check (T.is_obj t0) "bad annotation (object type expected)";
    let (s0, tfs0) = T.as_obj t0 in
    (type_obj env'' T.Actor fs) <: (T.Obj (s0, tfs0, []));
    t0 <: t;
  | NewObjE (s, fs, t0) ->
    (* check object *)
    let t1 = type_obj env s fs in
    check_typ env t1;

    (* check annotation *)
    check (T.is_obj t0) "bad annotation (object type expected)";
    let (s0, tfs0) = T.as_obj t0 in
    t1 <: T.Obj (s0, tfs0, []);

    t0 <: t
  end;
  (* check const annotation *)
  (* see ir_passes/const.ml for an explanation *)
  let check_var ctxt v =
    if not (T.Env.find v env.vals).const then
      error env exp.at "const %s with non-const variable %s" ctxt v in
  if exp.note.Note.const
  then begin
    match exp.it with
    | VarE (Const, id) -> check_var "VarE" id
    | FuncE (x, s, c, tp, as_ , ts, body) ->
      check (s = T.Local) "constant FuncE cannot be of shared sort";
      if env.lvl = NotTopLvl then
      Freevars.M.iter (fun v _ ->
        if (T.Env.find v env.vals).loc_known then () else
        check_var "FuncE" v
      ) (Freevars.exp exp)
    | NewObjE (Type.(Object | Module), fs, t) when T.is_immutable_obj t ->
      List.iter (fun f -> check_var "NewObjE" f.it.var) fs
    | PrimE (ArrayPrim (Const, _), es) ->
      List.iter (fun e1 ->
        check e1.note.Note.const "constant array with non-constant subexpression"
      ) es
    | PrimE (TupPrim, es) ->
      List.iter (fun e1 ->
        check e1.note.Note.const "constant tuple with non-constant subexpression"
      ) es
    | PrimE (DotPrim _, [e1]) ->
      check e1.note.Note.const "constant DotPrim on non-constant subexpression"
    | PrimE (ProjPrim _, [e1]) ->
      check e1.note.Note.const "constant ProjPrim on non-constant subexpression"
    | PrimE (OptPrim, [e1]) ->
      check e1.note.Note.const "constant OptPrim with non-constant subexpression"
    | PrimE (TagPrim _, [e1]) ->
      check e1.note.Note.const "constant TagPrim with non-constant subexpression"
    | BlockE (ds, e) ->
      List.iter (fun d -> match d.it with
        | VarD _ | RefD _ -> check false "VarD/RefD in constant BlockE"
        | LetD (p, e1) ->
          check (Ir_utils.is_irrefutable p) "refutable pattern in constant BlockE";
          check e1.note.Note.const "non-constant RHS in constant BlockE"
      ) ds;
      check e.note.Note.const "non-constant body in constant BlockE"
    | LitE _ -> ()
    | _ -> check false "unexpected constant expression"
  end;


and check_lexp env (lexp:Ir.lexp) : unit =
  (* helpers *)
  let check p = check env lexp.at p in
  let (<:) t1 t2 = check_sub env lexp.at t1 t2 in
  (* check type annotation *)
  let t = lexp.note in
  (match t with
  | T.Mut t -> check_typ env t
  | t -> error env lexp.at "lexp with non-mutable type");
  (* check typing *)
  match lexp.it with
  | VarLE id ->
    let { typ = t0; const; loc_known } =
      try T.Env.find id env.vals
      with Not_found -> error env lexp.at "unbound variable %s" id
    in
    check (not const) "cannot assign to constant variable";
    t0 <: t
  | DotLE (exp1, n) ->
    begin
      check_exp env exp1;
      let t1 = typ exp1 in
      let sort, tfs =
        try T.as_obj_sub [n] t1 with Invalid_argument _ ->
          error env exp1.at "expected object type, but expression produces type\n  %s"
            (T.string_of_typ_expand t1)
      in
      check (sort <> T.Actor) "sort mismatch";
      try T.lookup_val_field n tfs <: t with Invalid_argument _ ->
        error env exp1.at "field name %s does not exist in type\n  %s"
          n (T.string_of_typ_expand t1)
    end
  | IdxLE (exp1, exp2) ->
    check_exp env exp1;
    check_exp env exp2;
    let t1 = T.promote (typ exp1) in
    let t2 = try T.as_array_sub t1 with
             | Invalid_argument _ ->
               error env exp1.at "expected array type, but expression produces type\n  %s"
                                       (T.string_of_typ_expand t1)
    in
    typ exp2 <: T.nat;
    t2 <: t

(* Cases *)

and check_cases env t_pat t cases =
  List.iter (check_case env t_pat t) cases

and check_case env t_pat t {it = {pat; exp}; _} =
  let ve = check_pat env pat in
  check_sub env pat.at t_pat pat.note;
  check_exp (adjoin_vals env ve) exp;
  check env pat.at (sub env (typ exp) t) "bad case"

(* Arguments *)

and check_args env args =
  let rec go ve = function
    | [] -> ve
    | a::as_ ->
      if (T.Env.mem a.it ve) then
        error env a.at "duplicate binding for %s in argument list" a.it;
      check_typ env a.note;
      let val_info = {typ = a.note; const = false; loc_known = env.lvl = TopLvl } in
      let env' = T.Env.add a.it val_info ve in
      go env' as_
  in go T.Env.empty args

(* Patterns *)

and gather_pat env const ve0 pat : val_env =
  let rec go ve pat =
    match pat.it with
    | WildP
    | LitP _ ->
      ve
    | VarP id ->
      if T.Env.mem id ve0 then
        error env pat.at "duplicate binding for %s in block" id;
      let val_info = {typ = pat.note; const; loc_known = env.lvl = TopLvl} in
      T.Env.add id val_info ve (*TBR*)
    | TupP pats ->
      List.fold_left go ve pats
    | ObjP pfs ->
      List.fold_left go ve (pats_of_obj_pat pfs)
    | AltP (pat1, pat2) ->
      let ve1, ve2 = go ve pat1, go ve pat2 in
      let common i1 i2 = { typ = lub env i1.typ i2.typ; loc_known = i1.loc_known && i2.loc_known; const = i1.const && i2.const } in
      T.Env.merge (fun _ -> Lib.Option.map2 common) ve1 ve2
    | OptP pat1
    | TagP (_, pat1) ->
      go ve pat1
  in T.Env.adjoin ve0 (go T.Env.empty pat)

and check_pat_exhaustive env pat : val_env =
  let  ve = check_pat env pat in
  (* TODO: actually check exhaustiveness *)
  ve

and check_pat env pat : val_env =
  assert (pat.note <> T.Pre);
  let (<:) = check_sub env pat.at in
  let t = pat.note in
  match pat.it with
  | WildP -> T.Env.empty
  | VarP id -> T.Env.singleton id { typ = pat.note; const = false; loc_known = env.lvl = TopLvl }
  | LitP NullLit ->
    t <: T.Opt T.Any;
    T.Env.empty
  | LitP lit ->
    let t1 = T.Prim (type_lit env lit pat.at) in
    let t1' = if T.eq t1 T.nat then T.int else t1 in  (* account for subtyping *)
    t <: t1';
    T.Env.empty
  | TupP pats ->
    let ve = check_pats pat.at env pats T.Env.empty in
    let ts = List.map (fun pat -> pat.note) pats in
    t <: T.Tup ts;
    ve
  | ObjP pfs ->
    let ve = check_pats pat.at env (pats_of_obj_pat pfs) T.Env.empty in
    check_pat_fields env t pfs;
    ve
  | OptP pat1 ->
    let ve = check_pat env pat1 in
    t <: T.Opt pat1.note;
    ve
  | TagP (l, pat1) ->
    let ve = check_pat env pat1 in
    check_pat_tag env t l pat1;
    ve
  | AltP (pat1, pat2) ->
    let ve1, ve2 = check_pat env pat1, check_pat env pat2 in
    t <: pat1.note;
    t <: pat2.note;
    if T.Env.(keys ve1 <> keys ve2) then
        error env pat.at "set of bindings differ for alternative pattern";
    let common i1 i2 = { typ = lub env i1.typ i2.typ; loc_known = i1.loc_known && i2.loc_known; const = i1.const && i2.const } in
    T.Env.merge (fun _ -> Lib.Option.map2 common) ve1 ve2

and check_pats at env pats ve : val_env =
  match pats with
  | [] -> ve
  | pat::pats' ->
    let ve1 = check_pat env pat in
    let ve' = disjoint_union env at "duplicate binding for %s in pattern" ve ve1 in
    check_pats at env pats' ve'

and check_pat_fields env t = List.iter (check_pat_field env t)

and check_pat_field env t (pf : pat_field) =
  let lab = pf.it.name in
  let tf = T.{lab; typ = pf.it.pat.note; src = empty_src} in
  let s, tfs = T.as_obj_sub [lab] t in
  let (<:) = check_sub env pf.it.pat.at in
  t <: T.Obj (s, [tf], []);
  if T.is_mut (T.lookup_val_field lab tfs) then
    error env pf.it.pat.at "cannot match mutable field %s" lab

and check_pat_tag env t l pat =
  let (<:) = check_sub env pat.at in
  match T.lookup_val_field_opt l (T.as_variant_sub l t) with
  | Some t -> t <: pat.note
  | None -> ()

(* Objects *)

and type_obj env s fs : T.typ =
  let tfs = type_exp_fields env s fs in
  T.Obj (s, tfs, [])

and type_exp_fields env s fs : T.field list =
  let tfs = List.map (type_exp_field env s) fs in
  List.sort T.compare_field tfs

and type_exp_field env s f : T.field =
  let {name; var} = f.it in
  let { typ = t; const; loc_known } =
    try T.Env.find var env.vals
    with Not_found -> error env f.at "field typing for %s not found" name
  in
  assert (t <> T.Pre);
  check_sub env f.at t f.note;
  check env f.at ((s = T.Actor) ==> T.is_shared_func t)
    "public actor field must have shared function type";
  T.{lab = name; typ = t; src = empty_src}

(* Declarations *)

and check_open_typ_binds env typ_binds =
  let cs = List.map (fun tp -> tp.it.con) typ_binds in
  let ce = List.fold_right (fun c ce -> T.ConSet.disjoint_add c ce) cs T.ConSet.empty in
  let tbs = close_typ_binds cs (List.map (fun tb -> tb.it) typ_binds) in
  let _ = check_typ_binds env tbs in
  cs, tbs, ce

and check_open_typ_bind env typ_bind =
  match check_open_typ_binds env [typ_bind] with
  | [c], [tb], ce -> c, tb, ce
  | _ -> assert false

and close_typ_binds cs tbs =
  List.map (fun {con; sort; bound} -> {Type.var = Cons.name con; sort = sort; bound = Type.close cs bound}) tbs

and check_dec env dec  =
  (* helpers *)
  let (<:) t1 t2 = check_sub env dec.at t1 t2 in
  match dec.it with
  | LetD (pat, exp) ->
    ignore (check_pat_exhaustive env pat);
    check_exp env exp;
    typ exp <: pat.note
  | VarD (id, t, exp) ->
    check_exp env exp;
    typ exp <: t
  | RefD (id, t, lexp) ->
    check_lexp env lexp;
    lexp.note <: t

and check_decs env decs  =
  List.iter (check_dec env) decs;

and gather_block_decs env decs =
  List.fold_left (gather_dec env) empty_scope decs

and gather_dec env scope dec : scope =
  match dec.it with
  | LetD (pat, exp) ->
    { val_env = gather_pat env exp.note.Note.const scope.val_env pat }
  | VarD (id, t, exp) ->
    check_typ env t;
    check env dec.at
      (not (T.Env.mem id scope.val_env))
      "duplicate variable definition in block";
    let val_info = {typ = T.Mut t; const = false; loc_known = env.lvl = TopLvl} in
    { val_env = T.Env.add id val_info scope.val_env }
  | RefD (id, t, lexp) ->
    check_mut_typ env t;
    check env dec.at
      (not (T.Env.mem id scope.val_env))
      "duplicate variable definition in block";
    let val_info = {typ = t; const = false; loc_known = false} in
    { val_env = T.Env.add id val_info scope.val_env }

and to_async_sort = Type.(function | AwaitCmp -> Cmp | AwaitFut _ -> Fut)

(* Programs *)

let check_comp_unit env = function
  | LibU (ds, e) ->
    let scope = gather_block_decs env ds in
    let env' = adjoin env scope in
    check_decs env' ds;
    check_exp env' e;
  | ProgU ds ->
    let scope = gather_block_decs env ds in
    let env' = adjoin env scope in
    check_decs env' ds
  | ActorU (as_opt, ds, fs,
      { preupgrade; postupgrade; meta; heartbeat; timer; inspect; low_memory; stable_type; stable_record }, t0) ->
    let check p = check env no_region p in
    let (<:) t1 t2 = check_sub env no_region t1 t2 in
    let env' = match as_opt with
      | None -> { env with async = None }
      | Some as_ ->
        let ve = check_args env as_ in
        List.iter (fun a -> check_shared env no_region a.note) as_;
        adjoin_vals { env with async = None } ve
    in
    let scope1 = gather_block_decs env' ds in
    let env'' = adjoin env' scope1 in
    check_decs env'' ds;
    check_exp env'' preupgrade;
    check_exp env'' postupgrade;
    check_exp env'' heartbeat;
    check_exp env'' timer;
    check_exp env'' inspect;
    check_exp env'' stable_record;
    let async_cap = Some Async_cap.top_cap in
    check_exp { env'' with async = async_cap } low_memory;
    typ preupgrade <: T.unit;
    typ postupgrade <: T.unit;
    typ heartbeat <: T.unit;
    typ timer <: T.unit;
    typ inspect <: T.unit;
    typ low_memory <: T.unit;
    typ stable_record <: stable_type.post;
    check (T.is_obj t0) "bad annotation (object type expected)";
    let (s0, tfs0) = T.as_obj t0 in
    type_obj env'' T.Actor fs <: T.Obj (s0, tfs0, []);
    () (* t0 <: t *)

let check_prog verbose phase ((cu, flavor) as prog) : unit =
  let env = initial_env flavor in
  try check_comp_unit env cu
  with CheckFailed s ->
    let bt = Printexc.get_backtrace () in
    if verbose
    then begin
      Printf.eprintf "Ill-typed intermediate code after %s:\n" phase;
      Printf.eprintf "%s" (Wasm.Sexpr.to_string 80 (Arrange_ir.prog prog));
      Printf.eprintf "%s" s;
      Printf.eprintf "%s" bt;
    end else begin
      Printf.eprintf "Ill-typed intermediate code after %s (use -v to see dumped IR):\n" phase;
      Printf.eprintf "%s" s;
      Printf.eprintf "%s" bt;
    end;
    exit 1
