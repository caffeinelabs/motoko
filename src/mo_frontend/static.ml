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

let err m at =
  let open Diag in
  add_msg m
    (error_message
       at
       "M0014"
       "type"
       "non-static expression in library, module, migration expression, or actor body with enhanced migration.")

let pat_err m at =
  let open Diag in
  add_msg m
    (error_message
       at
       "M0015"
       "type"
       "only trivial patterns allowed in static expressions")

let rec exp ?(allow_system = false) m e  = match e.it with
  (* Plain values *)
  | HoleE (s, e) -> exp ~allow_system m !e 
  | (PrimE _ | LitE _ | ActorUrlE _ | FuncE _) -> ()
  | (TagE (_, exp1) | OptE exp1) -> exp ~allow_system m exp1 
  | TupE es -> List.iter (exp ~allow_system m) es
  | ArrayE (mut, es) ->
    begin
      match mut.it with
      | Const ->  List.iter (exp ~allow_system m) es
      | Var -> err m e.at
    end
  | ObjBlockE (eo, _, _, dfs) ->
    Option.iter (exp ~allow_system m) eo; dec_fields ~allow_system m dfs 
  | ObjE (bases, efs) ->
    List.iter (exp ~allow_system m) bases; exp_fields ~allow_system m efs

  (* Variable access. Dangerous, due to loops. *)
  | (VarE _ | ImportE _ | ImplicitLibE _) -> ()

  (* Projections. These are a form of evaluation. *)
  | ProjE (exp1, _)
  | DotE (exp1, _, _) -> exp ~allow_system m exp1
  | IdxE (exp1, exp2) -> err m e.at

  (* Transparent *)
  | AnnotE (exp1, _) | IgnoreE exp1 | DoOptE exp1 -> exp ~allow_system m exp1 
  | BlockE ds -> List.iter (dec ~allow_system m) ds

  (* 
     if <system> and we want to allow <system> calls, check.
     use-case: multi-migration actor bodies where we want to allow e.g., timers. 
  *)
  | CallE (_, _, inst, _) 
    -> (match (allow_system, inst.it) with 
        | (true, Some(true, _)) -> ()
        | _ -> err m e.at)
   
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
  -> err m e.at

and dec_fields ?(allow_system = false) m dfs = List.iter (fun df -> dec ~allow_system  m df.it.dec) dfs

and exp_fields ?(allow_system = false) m efs = List.iter (fun (ef : exp_field) ->
  if ef.it.mut.it = Var then err m ef.at;
  exp ~allow_system m ef.it.exp) efs

and dec ?(allow_system = false) m d = match d.it with
  | TypD _ | ClassD _ | MixinD _ -> ()
  | IncludeD _ when allow_system -> ()
  | ExpD e -> exp ~allow_system m e
  | LetD (p, e, fail) -> pat m p; exp ~allow_system m e; Option.iter (exp ~allow_system m) fail
  | VarD (_, e) when allow_system -> exp ~allow_system m e
  | VarD _ | IncludeD _ -> err m d.at

and pat m p = match p.it with
  | (WildP | VarP _) -> ()

  (*
  If we allow projections above, then we should allow irrefutable
  patterns here.
  *)
  | TupP ps -> List.iter (pat m) ps
  | ObjP fs -> List.iter (pat_field m) fs

  (* TODO:
    claudio: what about singleton variant patterns? These are irrefutable too.
    Andreas suggests simply allowing all patterns: "The worst that can happen is that the program
    is immediately terminated, but that doesn't break anything semantically."
  *)

  (* Everything else is forbidden *)
  | _ -> pat_err m p.at

and pat_field m pf = match pf.it with
  | ValPF(_, p) -> pat m p
  | TypPF(_) -> ()

let prog ?(allow_system = false) p =
  Diag.with_message_store (fun m -> List.iter (dec ~allow_system m) p.it; Some ())

let rec is_system e = match e.it with
  | CallE (_, _, inst, _) -> (match inst.it with Some (true, _) -> true | _ -> false)
  | IgnoreE e' | AnnotE (e', _) -> is_system e'
  | _ -> false

