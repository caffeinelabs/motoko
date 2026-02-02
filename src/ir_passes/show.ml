open Ir_def
open Mo_types
open Mo_values
(* Translates away calls to `show`. *)
open Source
open Ir
module T = Type
open Construct
open Typ_hash

(* Environment *)

(* We go through the file and collect all type arguments to `show`.
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

(* For a concrete type `t` we want to create a function name for `show`.
   This name needs to be disjoint from all user-generated names.
   Luckily, we are not limited in the characters to use at this point.
*)

let show_name_for t =
  "@show<" ^ typ_hash t ^ ">"

let show_fun_typ_for t =
  T.Func (T.Local, T.Returns, [], [t], [T.text])

let show_var_for t : Construct.var =
  var (show_name_for t) (show_fun_typ_for t)


(* Construction helpers *)

(* Many of these are simply the entry points for helper functions defined in
   the prelude. *)

let argVar t = var "x" t
let argE t = varE (argVar t)

let define_show : T.typ -> Ir.exp -> Ir.dec = fun t e ->
  Construct.funcD (show_var_for t) (argVar t) e

let invoke_generated_show : T.typ -> Ir.exp -> Ir.exp = fun t e ->
  varE (show_var_for t) -*- e

let invoke_prelude_show : string -> T.typ -> Ir.exp -> Ir.exp = fun n t e ->
  let fun_typ = T.Func (T.Local, T.Returns, [], [t], [T.text]) in
  varE (var n fun_typ) -*- argE t

let invoke_text_of_option : T.typ -> Ir.exp -> Ir.exp -> Ir.exp = fun t f e ->
  let fun_typ =
    T.Func (T.Local, T.Returns, [{T.var="T";T.sort=T.Type;T.bound=T.Any}], [show_fun_typ_for (T.Var ("T",0)); T.Opt (T.Var ("T",0))], [T.text]) in
  callE (varE (var "@text_of_option" fun_typ)) [t] (tupE [f; e])

let invoke_text_of_variant : T.typ -> Ir.exp -> T.lab -> Ir.exp -> Ir.exp = fun t f l e ->
  let fun_typ =
    T.Func (T.Local, T.Returns, [{T.var="T";T.sort=T.Type;T.bound=T.Any}], [T.text; show_fun_typ_for (T.Var ("T",0)); T.Var ("T",0)], [T.text]) in
  callE (varE (var "@text_of_variant" fun_typ)) [t] (tupE [textE l; f; e])

let invoke_text_of_array : T.typ -> Ir.exp -> Ir.exp -> Ir.exp = fun t f e ->
  let fun_typ =
    T.Func (T.Local, T.Returns, [{T.var="T";T.sort=T.Type;T.bound=T.Any}], [show_fun_typ_for (T.Var ("T",0)); T.Array (T.Var ("T",0))], [T.text]) in
  callE (varE (var "@text_of_array" fun_typ)) [t] (tupE [f; e])

let invoke_text_of_array_mut : T.typ -> Ir.exp -> Ir.exp -> Ir.exp = fun t f e ->
  let fun_typ =
    T.Func (T.Local, T.Returns, [{T.var="T";T.sort=T.Type;T.bound=T.Any}], [show_fun_typ_for (T.Var ("T",0)); T.Array (T.Mut (T.Var ("T",0)))], [T.text]) in
  callE (varE (var "@text_of_array_mut" fun_typ)) [t] (tupE [f; e])

let list_build : 'a -> (unit -> 'a) -> 'a -> 'a list -> 'a list = fun pre sep post xs ->
  let rec go = function
    | [] -> [ post ]
    | [x] -> [ x; post ]
    | x::xs -> [ x; sep () ] @ go xs
  in [ pre ] @ go xs

let catE : Ir.exp -> Ir.exp -> Ir.exp = fun e1 e2 ->
  { it = PrimE (BinPrim (T.text, Operator.CatOp), [e1; e2])
  ; at = no_region
  ; note = { Note.def with Note.typ = T.text }
  }

let cat_list : Ir.exp list -> Ir.exp = fun es ->
  List.fold_right catE es (textE "")

(* Synthesizing a single show function *)

(* Returns the new declarations, as well as a list of further types it needs *)


let show_for : T.typ -> Ir.dec * T.typ list = fun t ->
  match t with
  | T.(Prim Bool) ->
    define_show t (invoke_prelude_show "@text_of_Bool" t (argE t)),
    []
  | T.(Prim Nat) ->
    define_show t (invoke_prelude_show "@text_of_Nat" t (argE t)),
    []
  | T.(Prim Int) ->
    define_show t (invoke_prelude_show "@text_of_Int" t (argE t)),
    []
  | T.(Prim Nat8) ->
    define_show t (invoke_prelude_show "@text_of_Nat8" t (argE t)),
    []
  | T.(Prim Nat16) ->
    define_show t (invoke_prelude_show "@text_of_Nat16" t (argE t)),
    []
  | T.(Prim Nat32) ->
    define_show t (invoke_prelude_show "@text_of_Nat32" t (argE t)),
    []
  | T.(Prim Nat64) ->
    define_show t (invoke_prelude_show "@text_of_Nat64" t (argE t)),
    []
  | T.(Prim Int8) ->
    define_show t (invoke_prelude_show "@text_of_Int8" t (argE t)),
    []
  | T.(Prim Int16) ->
    define_show t (invoke_prelude_show "@text_of_Int16" t (argE t)),
    []
  | T.(Prim Int32) ->
    define_show t (invoke_prelude_show "@text_of_Int32" t (argE t)),
    []
  | T.(Prim Int64) ->
    define_show t (invoke_prelude_show "@text_of_Int64" t (argE t)),
    []
  | T.(Prim Float) ->
    define_show t (invoke_prelude_show "@text_of_Float" t (argE t)),
    []
  | T.(Prim Text) ->
    define_show t (invoke_prelude_show "@text_of_Text" t (argE t)),
    []
  | T.(Prim Char) ->
    define_show t (invoke_prelude_show "@text_of_Char" t (argE t)),
    []
  | T.(Prim Blob) ->
    define_show t (invoke_prelude_show "@text_of_Blob" t (argE t)),
    []
  | T.(Prim Principal) ->
    define_show t (primE IcUrlOfBlob [primE T.(CastPrim (principal, blob)) [argE t]]),
    []
  | T.(Prim Null) ->
    define_show t (textE "null"),
    []
  | T.Func _ ->
    define_show t (textE "func"),
    []
  | T.Con (c,_) ->
    (* t is normalized, so this is a type parameter *)
    define_show t (textE ("show_for: cannot handle type parameter " ^ T.string_of_typ t)),
    []
  | T.Tup [] ->
    define_show t (textE "()"),
    []
  | T.Tup ts' ->
    let ts' = List.map T.normalize ts' in
    define_show t (
      cat_list (list_build
        (textE "(") (fun () -> textE ", ") (textE ")")
        (List.mapi (fun i t' -> invoke_generated_show t' (projE (argE t) i)) ts')
      )
    ),
    ts'
  | T.Opt t' ->
    let t' = T.normalize t' in
    define_show t (invoke_text_of_option t' (varE (show_var_for t')) (argE t)),
    [t']
  | T.Weak t' ->
    let t' = T.normalize t' in
    define_show t (catE (textE ("weak "))
                        (invoke_text_of_option t' (varE (show_var_for t')) (primE (OtherPrim "weak_get")  [argE t]))),
    [t']
  | T.Array t' ->
    let t' = T.normalize t' in
    begin match t' with
    | T.Mut t' ->
      define_show t (invoke_text_of_array_mut t' (varE (show_var_for t')) (argE t)),
      [t']
    | _ ->
      define_show t (invoke_text_of_array t' (varE (show_var_for t')) (argE t)),
      [t']
    end
  | T.Obj (T.Object, fs, _) ->
    define_show t (
      cat_list (list_build
        (textE "{") (fun () -> textE "; ") (textE "}")
        (List.map (fun f ->
          let t' = T.as_immut (T.normalize f.Type.typ) in
          catE
            (textE (f.Type.lab ^ " = "))
            (invoke_generated_show t' (dotE (argE t) f.Type.lab t'))
          ) fs
        )
      )
    ),
    List.map (fun f -> T.as_immut (T.normalize (f.Type.typ))) fs
  | T.Variant fs ->
    define_show t (
      switch_variantE
        (argE t)
        (List.map (fun {T.lab = l; typ = t'; _} ->
          let t' = T.normalize t' in
          l,
          (varP (argVar t')), (* Shadowing, but that's fine *)
          (invoke_text_of_variant t' (varE (show_var_for t')) l (argE t'))
        ) fs)
        (T.text)
    ),
    List.map (fun (f : T.field) -> T.normalize f.T.typ) fs
  | T.Non ->
    define_show t (unreachableE ()),
    []
  | _ -> assert false (* Should be prevented by can_show *)

(* Synthesizing the types recursively. Hopefully well-founded. *)

let show_decls : T.typ M.t -> Ir.dec list = fun roots ->
  let seen = ref M.empty in

  let rec go = function
    | [] -> []
    | t::todo when M.mem (typ_hash t) !seen ->
      go todo
    | t::todo ->
      seen := M.add (typ_hash t) () !seen;
      let (decl, deps) = show_for t in
      decl :: go (deps @ todo)
  in go (List.map snd (M.bindings roots))

(* The AST traversal *)

(* Does two things:
 - collects all uses of `debug_show` in the `env`
 - for each actor, resets the environment, recurses,
   and adds the show functions (this keeps closed actors closed)
*)

let rec t_exps env decs = List.map (t_exp env) decs

and t_exp env (e : Ir.exp) =
  { e with it = t_exp' env e.it }

and t_exp' env = function
  | (LitE _ | VarE _) as e -> e
  | PrimE (ShowPrim ot, [exp1]) ->
    let t' = T.normalize ot in
    add_type env t';
    (varE (show_var_for t') -*- t_exp env exp1).it
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
          {it = {pat; exp = t_exp env exp}; at; note})
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
    let decls = show_decls !(env1.params) in
    ActorE (decls @ ds', fields,
      { meta;
        preupgrade = preupgrade';
        postupgrade = postupgrade';
        heartbeat = heartbeat';
        timer = timer';
        inspect = inspect';
        low_memory = low_memory';
        stable_record = stable_record';
        stable_type;
      },
      typ)

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
    let decls = show_decls !(env.params) in
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
    let decls = show_decls !(env.params) in
    ActorU (as_opt, decls @ ds', fields,
      { meta;
        preupgrade = preupgrade';
        postupgrade = postupgrade';
        heartbeat = heartbeat';
        timer = timer';
        inspect = inspect';
        low_memory = low_memory';
        stable_record = stable_record';
        stable_type
      }, typ)

(* Entry point for the program transformation *)

let transform (cu, flavor) =
  assert (not flavor.has_typ_field); (* required for hash_typ *)
  (t_comp_unit cu, {flavor with has_show = false})
