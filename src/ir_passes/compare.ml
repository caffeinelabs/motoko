(* Translates away comparison operators on structured types. *)

open Ir_def
open Mo_types
open Mo_values
open Source
open Ir
module T = Type
open Construct
open Typ_hash

(* Environment *)

(* We go through the file and collect all structured type arguments to ordering RelPrims.
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

let compare_name_for t =
  "@compare<" ^ typ_hash t ^ ">"

let compare_fun_typ_for t =
  T.Func (T.Local, T.Returns, [], [t; t], [T.int])

let compare_var_for t : Construct.var =
  var (compare_name_for t) (compare_fun_typ_for t)

(* Smart comparison constructor.
   Returns an Int expression: negative, zero, or positive.
   For singletons: always 0
   For primitives with backend support: emit RelPrim-based comparison
   For structured types: call the generated @compare<hash> function
*)
let compare_func_body : T.typ -> Ir.exp -> Ir.exp -> Ir.exp = fun t e1 e2 ->
  if T.singleton t
  then blockE [expD (ignoreE e1); expD (ignoreE e2)] (intE Numerics.Int.zero)
  else if Check_ir.has_prim_compare t
  then
    (* Encode primitive comparison as: if (e1 < e2) -1 else if (e1 == e2) 0 else 1 *)
    (* We need to be careful with side effects, so bind e1 and e2 first *)
    let v1 = var "v1" t in
    let v2 = var "v2" t in
    blockE
      [ letD v1 e1; letD v2 e2 ]
      (ifE (primE (RelPrim (t, Operator.LtOp)) [varE v1; varE v2])
        (intE (Numerics.Int.of_int (-1)))
        (ifE (primE (RelPrim (t, Operator.EqOp)) [varE v1; varE v2])
          (intE Numerics.Int.zero)
          (intE (Numerics.Int.of_int 1))))
  else varE (compare_var_for t) -*- (tupE [e1; e2])

(* Construction helpers *)

let arg1Var t = var "x1" t
let arg2Var t = var "x2" t
let arg1E t = varE (arg1Var t)
let arg2E t = varE (arg2Var t)

let define_compare : T.typ -> Ir.exp -> Ir.dec = fun t e ->
  Construct.nary_funcD (compare_var_for t) [arg1Var t; arg2Var t] e

let array_compare_func_body : T.typ -> Ir.exp -> Ir.exp -> Ir.exp -> Ir.exp = fun t f e1 e2 ->
  let fun_typ =
    T.Func (T.Local, T.Returns,
      [{T.var="T";T.sort=T.Type;T.bound=T.Any}],
      [compare_fun_typ_for (T.Var ("T",0)); T.Array (T.Var ("T",0)); T.Array (T.Var ("T",0))],
      [T.int]) in
  callE (varE (var "@compare_array" fun_typ)) [t] (tupE [f; e1; e2])

(* Lexicographic chaining helper: given a list of Int-typed expressions,
   return the first non-zero one, or 0 if all are zero.
   chain [e1; e2; e3] => let c1 = e1 in if c1 != 0 then c1 else let c2 = e2 in ...
*)
let rec lex_chain : Ir.exp list -> Ir.exp = function
  | [] -> intE Numerics.Int.zero
  | [e] -> e
  | e :: es ->
    let c = var "c" T.int in
    letE c e
      (ifE (primE (RelPrim (T.int, Operator.EqOp)) [varE c; intE Numerics.Int.zero])
        (lex_chain es)
        (varE c))

(* Synthesizing a single compare function *)

(* Returns the new declaration, as well as a list of further types it needs *)

let compare_for : T.typ -> Ir.dec * T.typ list = fun t ->
  match t with
  (* Function wrappers around primitive types *)
  | t when T.singleton t || Check_ir.has_prim_compare t ->
    define_compare t (compare_func_body t (arg1E t) (arg2E t)),
    []
  (* Error cases *)
  | T.Con (c,_) ->
    raise (Invalid_argument ("compare_for: cannot handle type parameter " ^ T.string_of_typ t))
  (* Structured types *)
  | T.Tup ts' ->
    let ts' = List.map T.normalize ts' in
    define_compare t (
      lex_chain (List.mapi (fun i t' ->
        compare_func_body t' (projE (arg1E t) i) (projE (arg2E t) i)
      ) ts')
    ),
    ts'
  | T.Opt t' ->
    let t' = T.normalize t' in
    let y1 = var "y1" t' in
    let y2 = var "y2" t' in
    define_compare t (switch_optE (arg1E t)
      (* x1 is null *)
      ( switch_optE (arg2E t)
        (* x2 is null: null vs null => 0 *)
        (intE Numerics.Int.zero)
        (* x2 is ?_: null < ?_ => -1 *)
        wildP (intE (Numerics.Int.of_int (-1)))
        T.int
      )
      (* x1 is ?y1 *)
      ( varP y1 )
      ( switch_optE (arg2E t)
        (* x2 is null: ?_ > null => 1 *)
        (intE (Numerics.Int.of_int 1))
        (* x2 is ?y2: compare contents *)
        ( varP y2 )
        ( compare_func_body t' (varE y1) (varE y2) )
        T.int
      )
      T.int
    ),
    [t']
  | T.Array t' ->
    begin match T.normalize t' with
    | T.Mut _ -> assert false (* mutable arrays not orderable *)
    | t' ->
      define_compare t (array_compare_func_body t' (varE (compare_var_for t')) (arg1E t) (arg2E t)),
      [t']
    end
  | T.Obj ((T.Object | T.Memory | T.Module), fs, _) ->
    let sorted_fs = List.sort T.compare_field fs in
    define_compare t (
      lex_chain (List.map (fun f ->
        let t' = T.as_immut (T.normalize f.Type.typ) in
        compare_func_body t' (dotE (arg1E t) f.Type.lab t') (dotE (arg2E t) f.Type.lab t')
      ) sorted_fs)
    ),
    List.map (fun f -> T.as_immut (T.normalize (f.Type.typ))) sorted_fs
  | T.Variant fs ->
    (* For variants: first compare tags alphabetically, then if same tag compare payloads *)
    let sorted_fs = List.sort T.compare_field fs in
    define_compare t (
      { it = SwitchE
        ( tupE [arg1E t; arg2E t],
          (* Cases for matching tags *)
          List.map (fun f ->
            let t' = T.normalize f.Type.typ in
            let y1 = var "y1" t' in
            let y2 = var "y2" t' in
            { it = {
                pat = { it = TupP
                  [ { it = TagP (f.Type.lab, varP y1); at = no_region; note = t }
                  ; { it = TagP (f.Type.lab, varP y2); at = no_region; note = t }
                  ]; at = no_region; note = T.Tup [t;t] };
                exp = compare_func_body t' (varE y1) (varE y2);
              }; at = no_region; note = ()
            }) sorted_fs @
          (* Wildcard: different tags. We need to determine order by tag name.
             For the interpreter-level pass, we extract tags and compare them.
             At the IR level, we encode this as a chain of switches:
             switch x1 { #tag1 _ => switch x2 { #tag1 _ => unreachable (handled above), #tag2 _ => -1, ... } ... }
             This is complex. A simpler approach: assign each tag a numeric index and compare.
          *)
          (* Simple approach: for mismatched tags, switch on x1 to get its index,
             switch on x2 to get its index, then compare the indices. *)
          [ { it = {
                pat = wildP;
                exp =
                  let idx1 = var "idx1" T.int in
                  let idx2 = var "idx2" T.int in
                  let tag_index_switch arg =
                    { it = SwitchE
                      ( arg,
                        List.mapi (fun i f ->
                          { it = {
                              pat = { it = TagP (f.Type.lab, wildP); at = no_region; note = t };
                              exp = intE (Numerics.Int.of_int i);
                            }; at = no_region; note = ()
                          }) sorted_fs @
                        [ { it = { pat = wildP; exp = unreachableE () };
                            at = no_region; note = () } ]
                      );
                    at = no_region;
                    note = Note.{ def with typ = T.int }
                    }
                  in
                  blockE
                    [ letD idx1 (tag_index_switch (arg1E t));
                      letD idx2 (tag_index_switch (arg2E t)) ]
                    (compare_func_body T.int (varE idx1) (varE idx2))
              }; at = no_region; note = () } ]
        );
      at = no_region;
      note = Note.{ def with typ = T.int }
      }
    ),
    List.map (fun (f : T.field) -> T.normalize f.T.typ) sorted_fs
  | T.Non ->
    define_compare t (unreachableE ()),
    []
  | t ->
    raise (Invalid_argument ("Ir_passes.Compare.compare_for: Unexpected type " ^ T.string_of_typ t))

(* Synthesizing the types recursively. *)

let compare_decls : T.typ M.t -> Ir.dec list = fun roots ->
  let seen = ref M.empty in

  let rec go = function
    | [] -> []
    | t::todo when M.mem (typ_hash t) !seen ->
      go todo
    | t::todo ->
      seen := M.add (typ_hash t) () !seen;
      let (decl, deps) = compare_for t in
      decl :: go (deps @ todo)
  in go (List.map snd (M.bindings roots))

(* The AST traversal *)

let is_ordering_op = function
  | Operator.LtOp | Operator.GtOp | Operator.LeOp | Operator.GeOp -> true
  | _ -> false

let relop_to_int_cmp = function
  | Operator.LtOp -> Operator.LtOp
  | Operator.GtOp -> Operator.GtOp
  | Operator.LeOp -> Operator.LeOp
  | Operator.GeOp -> Operator.GeOp
  | _ -> assert false

let rec t_exps env = List.map (t_exp env)

and t_exp env (e : Ir.exp) =
  { e with it = t_exp' env e.it }

and t_exp' env = function
  | (LitE _ | VarE _) as e -> e
  | PrimE (RelPrim (ot, op), [exp1; exp2]) when is_ordering_op op && T.singleton ot ->
    let e1 = t_exp env exp1 in
    let e2 = t_exp env exp2 in
    (* singleton types are always equal, so < and > are false, <= and >= are true *)
    let result = match op with
      | Operator.LeOp | Operator.GeOp -> trueE ()
      | Operator.LtOp | Operator.GtOp -> falseE ()
      | _ -> assert false
    in
    (blockE [expD (ignoreE e1); expD (ignoreE e2)] result).it
  | PrimE (RelPrim (ot, op), [exp1; exp2]) when is_ordering_op op && not (Check_ir.has_prim_compare ot) ->
    let t' = T.normalize ot in
    add_type env t';
    let cmp_result = varE (compare_var_for t') -*- (tupE [t_exp env exp1; t_exp env exp2]) in
    let int_op = relop_to_int_cmp op in
    (primE (RelPrim (T.int, int_op)) [cmp_result; intE Numerics.Int.zero]).it
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
    let env1 = empty_env () in
    let ds' = t_decs env1 ds in
    let preupgrade' = t_exp env1 preupgrade in
    let postupgrade' = t_exp env1 postupgrade in
    let heartbeat' = t_exp env1 heartbeat in
    let timer' = t_exp env1 timer in
    let inspect' = t_exp env1 inspect in
    let low_memory' = t_exp env1 low_memory in
    let stable_record' = t_exp env1 stable_record in
    let decls = compare_decls !(env1.params) in
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
    let decls = compare_decls !(env.params) in
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
    let decls = compare_decls !(env.params) in
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
  (t_comp_unit cu, {flavor with has_poly_compare = false})
