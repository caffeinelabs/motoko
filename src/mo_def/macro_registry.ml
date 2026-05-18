(* Macro expander registry for `prim type` definitions.

   The registry is keyed by the `prim type` alias name.  Each entry is
   an [expander option ref]:

     1. Parser sees [prim type X<…>(…) = …]
          → [allocate "X"] creates a [ref None] under that key.
     2. Typechecker reaches [check_typ_prim_def] for the same X
          → on success, [set "X" expander] writes [Some expander]
            into the existing ref.
     3. Parser sees [switch type T { case <typ> <exp>; … }]
          → [get "X"] retrieves the (now-populated) ref, asserts the
            slot is filled, invokes the expander, and splices the
            output into the surrounding [SwitchE].

   The expander record has two callable fields:

   - [scrut_of u] takes the user's scrutinee [u] (e.g. the type-binder
     name as a value) and returns the actual [SwitchE] scrutinee
     (typically [@typCode u] with the prim type's discr template
     substituted).

   - [cases_of legs] takes the user's [switch type] legs and returns
     the compiled [case list] — each [arm.pat] reified as an [IntLit]
     pattern, each leg body wrapped in [RefineE] with the arm's
     refinement clause.

   The [build_expander] constructor lives in [mo_frontend]
   (it needs [Traversals.over_exp] for substitution). *)

open Syntax

type expander = {
  scrut_of : exp -> exp;
  cases_of : id -> case list -> case list;
    (* [cases_of user_id legs] compiles the legs and renames each
       refinement's [tv] from the prim-type's placeholder (e.g. `@T`)
       to [user_id] so the resulting [RefineE] σ targets the user's
       in-scope type-binder. *)
}

let registry : (string, expander option ref) Hashtbl.t = Hashtbl.create 8

let allocate (name : string) : expander option ref =
  match Hashtbl.find_opt registry name with
  | Some r -> r
  | None ->
    let r = ref None in
    Hashtbl.add registry name r;
    r

let set (name : string) (e : expander) : unit =
  let r = allocate name in
  r := Some e

let get (name : string) : expander option ref option =
  Hashtbl.find_opt registry name

let reset () = Hashtbl.reset registry
