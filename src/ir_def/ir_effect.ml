open Ir
open Source
module T = Mo_types.Type

(* a simple effect analysis to annote expressions as Triv(ial) (await-free) or Await (containing unprotected awaits) *)

(* in future we could merge this with the type-checker
   but I prefer to keep it mostly separate for now *)

let max_eff e1 e2 =
  match e1, e2 with
  | T.Triv, T.Triv -> T.Triv
  | _ , T.Await -> T.Await
  | T.Await, _ -> T.Await

let max_effs' seed = List.fold_left max_eff seed
let max_effs es = max_effs' T.Triv es
let map_max_effs' seed f l = max_effs' seed (List.map f l)
let map_max_effs f l = map_max_effs' T.Triv f l

let typ phrase = phrase.note.Note.typ
let eff phrase = phrase.note.Note.eff

let is_triv phrase = eff phrase = T.Triv

let effect_exp (exp: exp) : T.eff = eff exp

let is_async_call p exps =
  match p, exps with
  | CallPrim _, [exp1; _] ->
    T.is_shared_func (typ exp1) ||
    T.is_local_async_func (typ exp1)
  | OtherPrim "call_raw", _ ->
    true
  | _ -> false

(* infer the effect of an expression, assuming all sub-expressions are correctly effect-annotated *)

let rec infer_effect_prim p exps =
  match p, exps with
  | ThrowPrim, _
  | AwaitPrim _, _ ->
    T.Await
  | _ ->
    if is_async_call p exps then
      T.Await
    else
      map_max_effs eff exps

and infer_effect_exp (exp: exp) : T.eff =
  match exp.it with
  | VarE _
  | LitE _ ->
    T.Triv
  | LabelE (_, _, exp1)
  | LoopE exp1
  | AssignE (_, exp1) ->
    effect_exp exp1
  | PrimE (p, exps) ->
    infer_effect_prim p exps
  | BlockE (ds, exp) ->
    map_max_effs' (effect_exp exp) effect_dec ds
  | IfE (exp1, exp2, exp3) ->
    let e1 = effect_exp exp1 in
    let e2 = effect_exp exp2 in
    let e3 = effect_exp exp3 in
    max_eff e1 (max_eff e2 e3)
  | SwitchE (exp1, cases) ->
    let e1 = effect_exp exp1 in
    let e2 = effect_cases cases in
    max_eff e1 e2
  | AsyncE (T.Fut, _, _, _) ->
    T.Await
  | AsyncE (T.Cmp, _, _, _) ->
    T.Triv
  | TryE _ ->
    T.Await
  | DeclareE (_, _, exp1) ->
    effect_exp exp1
  | DefineE (_, _, exp1) ->
    effect_exp exp1
  | FuncE _ ->
    T.Triv
  | SelfCallE (_, _, exp1, exp2, exp3) ->
    let e1 = effect_exp exp1 in
    let e2 = effect_exp exp2 in
    let e3 = effect_exp exp3 in
    max_eff e1 (max_eff e2 e3)
  | ActorE _ ->
    T.Triv
  | NewObjE _ ->
    T.Triv

and effect_cases cases =
  match cases with
  | [] ->
    T.Triv
  | {it = {pat; exp}; _}::cases' ->
    let e = effect_exp exp in
    max_eff e (effect_cases cases')

and effect_dec dec = match dec.it with
  | LetD (_, e) | VarD (_, _, e) -> effect_exp e
  | RefD (_, _, { it = DotLE (e, _); _ }) -> effect_exp e
  | RefD (_, _, _) -> assert false

let infer_effect_dec = effect_dec

let infer_effect_decs = map_max_effs effect_dec
