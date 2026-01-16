(* Translates away equality on structured types. *)

open Ir_def
open Mo_types
open Mo_values
open Source
open Ir
module T = Type
open Construct
open Typ_hash

(* Environment *)

(* We go through the file and collect all structured type arguments to `OpEq`.
   We store them in `params`, indexed by their `type_id`
*)

module M = Map.Make(String)
type env =
  { params : T.typ M.t ref
  }

let empty_env () : env = {
  params = ref M.empty;
  }

let add_type env t : unit =
  env.params := M.add (typ_hash t) t !(env.params)

(* Function names *)

(* For a concrete type `t` we want to create a function name for `eq`.
   This name needs to be disjoint from all user-generated names.
   Luckily, we are not limited in the characters to use at this point.
*)

let eq_name_for t =
  "@eq<" ^ typ_hash t ^ ">"

let eq_fun_typ_for t =
  T.Func (T.Local, T.Returns, [], [t; t], [T.bool])

let eq_var_for t : Construct.var =
  var (eq_name_for t) (eq_fun_typ_for t)


(* Smart equality constructor. *)

(* This handles singletons directly, leaves primitives as Prims,
   and calls the right eq_var_for otherwise
*)

let eq_func_body : T.typ -> Ir.exp -> Ir.exp -> Ir.exp = fun t e1 e2 ->
  (* This is used when constructing equality functions, so e1 e2
     are pure (variables, projections) and can be dropped *)
  if T.singleton t
  then blockE [expD (ignoreE e1); expD (ignoreE e2)] (trueE ())
  else if Check_ir.has_prim_eq t
  then primE (RelPrim (t, Operator.EqOp)) [e1; e2]
  else varE (eq_var_for t) -*- (tupE [e1; e2])

(* Construction helpers *)

let arg1Var t = var "x1" t
let arg2Var t = var "x2" t
let arg1E t = varE (arg1Var t)
let arg2E t = varE (arg2Var t)

let define_eq : T.typ -> Ir.exp -> Ir.dec = fun t e ->
  Construct.nary_funcD (eq_var_for t) [arg1Var t; arg2Var t] e

let array_eq_func_body : T.typ -> Ir.exp -> Ir.exp -> Ir.exp -> Ir.exp = fun t f e1 e2 ->
  let fun_typ =
    T.Func (T.Local, T.Returns, [{T.var="T";T.sort=T.Type;T.bound=T.Any}], [eq_fun_typ_for (T.Var ("T",0)); T.Array (T.Var ("T",0)); T.Array (T.Var ("T",0))], [T.bool]) in
  callE (varE (var "@equal_array" fun_typ)) [t] (tupE [f; e1; e2])

(* Synthesizing a single show function *)

(* Returns the new declarations, as well as a list of further types it needs *)

let eq_for : T.typ -> Ir.dec * T.typ list = fun t ->
  match t with
  (* Function wrappers around primitive types *)
  (* These are needed when one of these types appears in an array, as we
     need a function to pass to @equal_array *)
  | t when T.singleton t || Check_ir.has_prim_eq t ->
    define_eq t (eq_func_body t (arg1E t) (arg2E t)),
    []
  (* Error cases *)
  | T.Con (c,_) ->
    (* t is normalized, so this is a type parameter *)
    raise (Invalid_argument ("eq_for: cannot handle type parameter " ^ T.string_of_typ t))
  (* Structured types *)
  | T.Tup ts' ->
    let ts' = List.map T.normalize ts' in
    define_eq t (
      conjE (List.mapi (fun i t' ->
        eq_func_body t' (projE (arg1E t) i) (projE (arg2E t) i)
      ) ts')
    ),
    ts'
  | T.Opt t' ->
    let t' = T.normalize t' in
    let y1 = var "y1" t' in
    let y2 = var "y2" t' in
    define_eq t (switch_optE (arg1E t)
      (* x1 is null *)
      ( switch_optE (arg2E t)
        (* x2 is null *)
        (trueE ())
        (* x2 is ?_ *)
        wildP (falseE ())
        (* ret type *)
        T.bool
      ) (* x1 is ?y1 *)
      ( varP y1 )
      ( switch_optE (arg2E t)
        (* x2 is null *)
        (falseE ())
        (* x2 is ?_ *)
        ( varP y2 )
        ( eq_func_body t' (varE y1) (varE y2) )
        (* ret type *)
        T.bool
      )
      (* ret type *)
      T.bool
    ),
    [t']
  | T.Array t' ->
    begin match T.normalize t' with
    | T.Mut _ -> assert false (* mutable arrays not equatable *)
    | t' ->
      define_eq t (array_eq_func_body t' (varE (eq_var_for t')) (arg1E t) (arg2E t)),
      [t']
    end
  | T.Obj ((T.Object | T.Memory | T.Module), fs, _) ->
    define_eq t (
      conjE (List.map (fun f ->
        let t' = T.as_immut (T.normalize f.Type.typ) in
        eq_func_body t' (dotE (arg1E t) f.Type.lab t') (dotE (arg2E t) f.Type.lab t')
      ) fs)
    ),
    List.map (fun f -> T.as_immut (T.normalize (f.Type.typ))) fs
  | T.Variant fs ->
    define_eq t (
      (* switching on the diagonal *)
      { it = SwitchE
        ( tupE [arg1E t; arg2E t],
          List.map (fun f ->
            let t' = T.normalize f.Type.typ in
            let y1 = var "y1" t' in
            let y2 = var "y2" t' in
            { it = {
                pat = { it = TupP
                  [ { it = TagP (f.Type.lab, varP y1); at = no_region; note = t }
                  ; { it = TagP (f.Type.lab, varP y2); at = no_region; note = t }
                  ]; at = no_region; note = T.Tup [t;t] };
                exp = eq_func_body t' (varE y1) (varE y2);
              }; at = no_region; note = ()
            }) fs @
            [ { it = { pat = wildP; exp = falseE () };
                at = no_region; note = () } ]
        );
      at = no_region;
      note = Note.{ def with typ = T.bool }
      }
    ),
    List.map (fun (f : T.field) -> T.normalize f.T.typ) fs
  | T.Non ->
    define_eq t (unreachableE ()),
    []
  | t ->
    raise (Invalid_argument ("Ir_passes.Eq.eq_on: Unexpected type " ^ T.string_of_typ t))

(* Synthesizing the types recursively. Hopefully well-founded. *)

let eq_decls : T.typ M.t -> Ir.dec list = fun roots ->
  let seen = ref M.empty in

  let rec go = function
    | [] -> []
    | t::todo when M.mem (typ_hash t) !seen ->
      go todo
    | t::todo ->
      seen := M.add (typ_hash t) () !seen;
      let (decl, deps) = eq_for t in
      decl :: go (deps @ todo)
  in go (List.map snd (M.bindings roots))

(* The AST traversal *)

(* Does two things:
 - collects all structured uses of `OpEq` in the `env`
 - for each actor, resets the environment, recurses,
   and adds the equality functions (this keeps closed actors closed)
*)

let rec t_exps env = List.map (t_exp env)

and t_exp env (e : Ir.exp) =
  { e with it = t_exp' env e.it }

and t_exp' env = function
  | (LitE _ | VarE _) as e -> e
  | PrimE (RelPrim (ot, Operator.EqOp), [exp1; exp2]) when T.singleton ot ->
    (* Handle singletons here, but beware of side-effects *)
    let e1 = t_exp env exp1 in
    let e2 = t_exp env exp2 in
    (blockE [expD (ignoreE e1); expD (ignoreE e2)] (trueE ())).it
  | PrimE (RelPrim (ot, Operator.EqOp), [exp1; exp2]) when not (Check_ir.has_prim_eq ot) ->
    let t' = T.normalize ot in
    add_type env t';
    (varE (eq_var_for t') -*- (tupE [t_exp env exp1; t_exp env exp2])).it
  | PrimE (p, es) -> PrimE (p, t_exps env es)
  | AssignE (lexp1, exp2) ->
    AssignE (t_lexp env lexp1, t_exp env exp2)
  | FuncE (s, c, id, typbinds, pat, typT, exp) ->
    FuncE (s, c, id, typbinds, pat, typT, t_exp env exp)
  | BlockE block -> BlockE (t_block env block)
  | IfE (exp1, exp2, exp3) ->
    IfE (t_exp env exp1, t_exp env exp2, t_exp env exp3)
  | SwitchE (exp1, cases) ->
    let cases' =
      List.map
        (fun {it = {pat;exp}; at; note} ->
          {it = {pat = pat; exp = t_exp env exp}; at; note})
        cases
    in
    SwitchE (t_exp env exp1, cases')
  | TryE (exp1, cases, vt) ->
    let cases' =
      List.map
        (fun {it = {pat;exp}; at; note} ->
          {it = {pat = pat; exp = t_exp env exp}; at; note})
        cases
    in
    TryE (t_exp env exp1, cases', vt)
  | LoopE exp1 ->
    LoopE (t_exp env exp1)
  | LabelE (id, typ, exp1) ->
    LabelE (id, typ, t_exp env exp1)
  | AsyncE (s, tb, e, typ) -> AsyncE (s, tb, t_exp env e, typ)
  | DeclareE (id, typ, exp1) ->
    DeclareE (id, typ, t_exp env exp1)
  | DefineE (id, mut ,exp1) ->
    DefineE (id, mut, t_exp env exp1)
  | NewObjE (sort, ids, t) ->
    NewObjE (sort, ids, t)
  | SelfCallE (ts, e1, e2, e3, e4) ->
    SelfCallE (ts, t_exp env e1, t_exp env e2, t_exp env e3, t_exp env e4)
  | ActorE (ds, fields, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, typ) ->
    (* Until Actor expressions become their own units,
       we repeat what we do in `comp_unit` below *)
    let env1 = empty_env () in
    let ds' = t_decs env1 ds in
    let preupgrade' = t_exp env1 preupgrade in
    let postupgrade' = t_exp env1 postupgrade in
    let heartbeat' = t_exp env1 heartbeat in
    let timer' = t_exp env1 timer in
    let inspect' = t_exp env1 inspect in
    let low_memory' = t_exp env1 low_memory in
    let stable_record' = t_exp env1 stable_record in
    let decls = eq_decls !(env1.params) in
    ActorE (decls @ ds', fields,
      {meta;
       preupgrade = preupgrade';
       postupgrade = postupgrade';
       heartbeat = heartbeat';
       timer = timer';
       inspect = inspect';
       low_memory = low_memory';
       stable_record = stable_record';
       stable_type;
      },
      typ
      )

and t_lexp env (e : Ir.lexp) = { e with it = t_lexp' env e.it }
and t_lexp' env = function
  | VarLE id -> VarLE id
  | IdxLE (exp1, exp2) ->
    IdxLE (t_exp env exp1, t_exp env exp2)
  | DotLE (exp1, n) ->
    DotLE (t_exp env exp1, n)

and t_dec env dec = { dec with it = t_dec' env dec.it }

and t_dec' env dec' =
  match dec' with
  | LetD (pat,exp) -> LetD (pat,t_exp env exp)
  | VarD (id, typ, exp) -> VarD (id, typ, t_exp env exp)
  | RefD (id, typ, lexp) -> RefD (id, typ, t_lexp env lexp)

and t_decs env decs = List.map (t_dec env) decs

and t_block env (ds, exp) = (t_decs env ds, t_exp env exp)

and t_comp_unit = function
  | LibU _ -> raise (Invalid_argument "cannot compile library")
  | ProgU ds ->
    let env = empty_env () in
    let ds' = t_decs env ds in
    let decls = eq_decls !(env.params) in
    ProgU (decls @ ds')
  | ActorU (as_opt, ds, fields, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, typ) ->
    let env = empty_env () in
    let ds' = t_decs env ds in
    let preupgrade' = t_exp env preupgrade in
    let postupgrade' = t_exp env postupgrade in
    let heartbeat' = t_exp env heartbeat in
    let timer' = t_exp env timer in
    let inspect' = t_exp env inspect in
    let low_memory' = t_exp env low_memory in
    let stable_record' = t_exp env stable_record in
    let decls = eq_decls !(env.params) in
    ActorU (as_opt, decls @ ds', fields,
      {meta;
       preupgrade = preupgrade';
       postupgrade = postupgrade';
       heartbeat = heartbeat';
       timer = timer';
       inspect = inspect';
       low_memory = low_memory';
       stable_record = stable_record';
       stable_type;
      }, typ)

(* Entry point for the program transformation *)

let transform (cu, flavor) =
  assert (not flavor.has_typ_field); (* required for hash_typ *)
  (t_comp_unit cu, {flavor with has_poly_eq = false})
