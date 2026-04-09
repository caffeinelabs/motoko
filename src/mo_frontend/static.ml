(*
This module implements the staticity check, needed for modules and imported
files.

The guiding principle is: Static expressions are expressions that can be
compiled to values without evaluation.

There is some mushiness around let-expressions and variables, which do form
some kind of beta-reduction, and can actually cause loops, but are required to
allow re-exporting names in modules.
*)

open Mo_def
open Source
open Syntax

type env = { msg_store : Diag.msg_store;
             allow_var : bool;
             allow_sys_call : bool;
             allow_include : bool;
           }

let err env at =
  let open Diag in
  add_msg env.msg_store
   (error_message
      at
      "M0014"
      "type"
      (if not (env.allow_sys_call) then
         "non-static expression in library, module or migration expression"
       else
         "non-static expression in actor body compiled with enhanced migration capabilities."))

let pat_err env at =
  let open Diag in
  add_msg env.msg_store
    (error_message
       at
       "M0015"
       "type"
       "only trivial patterns allowed in static expressions")

let rec exp env e  = match e.it with
  (* Plain values *)
  | HoleE (s, e) -> exp env !e
  | (PrimE _ | LitE _ | ActorUrlE _ | FuncE _) -> ()
  | (TagE (_, exp1) | OptE exp1) -> exp env exp1
  | TupE es -> List.iter (exp env) es
  | ArrayE (mut, es) ->
    begin
      match mut.it with
      | Const ->  List.iter (exp env) es
      | Var -> err env e.at
    end
  | ObjBlockE (eo, _, _, dfs) ->
    Option.iter (exp env) eo; dec_fields env dfs
  | ObjE (bases, efs) ->
    List.iter (exp env) bases; exp_fields env efs

  (* Variable access. Dangerous, due to loops. *)
  | (VarE _ | ImportE _ | ImplicitLibE _) -> ()

  (* Projections. These are a form of evaluation. *)
  | ProjE (exp1, _)
  | DotE (exp1, _, _) -> exp env exp1
  | IdxE (exp1, exp2) -> err env e.at

  (* Transparent *)
  | AnnotE (exp1, _) | IgnoreE exp1 | DoOptE exp1 -> exp env exp1
  | BlockE ds -> List.iter (dec env) ds

  (*
     if <system> and we want to allow <system> calls, check.
     use-case: multi-migration actor bodies where we want to allow e.g., timers.
  *)
  | CallE (_, callee, inst, (_, ref_args))
    -> (match (env.allow_sys_call, inst.it) with
        | (true, Some(true, _)) ->
          (exp env callee;
           exp env !ref_args)
        | _ -> err env e.at)

  (* Clearly non-static *)
  | UnE _
  | ShowE _
  | ToCandidE _
  | FromCandidE _
  | NotE _
  | AssertE _
  | LabelE _
  | BreakE _
  | RetE _
  | AsyncE _ (* TBR - Cmp could be static *)
  | AwaitE _
  | LoopE _
  | BinE _
  | RelE _
  | AssignE _
  | AndE _
  | OrE _
  | WhileE _
  | ForE _
  | DebugE _
  | IfE _
  | SwitchE _
  | ThrowE _
  | TryE _
  | BangE _
  -> err env e.at

and dec_fields env dfs = List.iter (fun df -> dec env df.it.dec) dfs

and exp_fields env efs = List.iter (fun (ef : exp_field) ->
  if ef.it.mut.it = Var then err env ef.at;
  exp env ef.it.exp) efs

and dec env d = match d.it with
  | TypD _ | ClassD _ | MixinD _ -> ()
  | IncludeD _ ->
    if env.allow_include
    then ()
    else err env d.at
  | ExpD e -> exp env e
  | LetD (p, e, fail) ->
    pat env p;
    exp env e;
    Option.iter (exp env) fail
  | VarD (_, e) ->
    if env.allow_var
    then exp env e
    else err env d.at

and pat env p = match p.it with
  | (WildP | VarP _) -> ()

  (*
  If we allow projections above, then we should allow irrefutable
  patterns here.
  *)
  | TupP ps -> List.iter (pat env) ps
  | ObjP fs -> List.iter (pat_field env) fs

  (* TODO:
    claudio: what about singleton variant patterns? These are irrefutable too.
    Andreas suggests simply allowing all patterns: "The worst that can happen is that the program
    is immediately terminated, but that doesn't break anything semantically."
  *)

  (* Everything else is forbidden *)
  | _ -> pat_err env p.at

and pat_field env pf = match pf.it with
  | ValPF(_, p) -> pat env p
  | TypPF(_) -> ()

let module_fields msg_store =
  dec_fields {
    msg_store;
    allow_var = false;
    allow_sys_call = false;
    allow_include = false }
let exp msg_store =
  exp
    { msg_store;
      allow_var = false;
      allow_sys_call = false;
      allow_include = false }

let actor_fields msg_store =
  dec_fields {
    msg_store;
    allow_var = true;
    allow_sys_call = true;
    allow_include = true }

let prog p =
  Diag.with_message_store (fun msg_store ->
      Some (List.iter (dec
       {msg_store;
        allow_var = false;
        allow_sys_call = false;
        allow_include = false })
       p.it))
