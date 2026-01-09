open Mo_types
open Mo_values

open Operator


(* Notes *)

type typ_note = {note_typ : Type.typ; note_eff : Type.eff}

let empty_typ_note = {note_typ = Type.Pre; note_eff = Type.Triv}

(* Resolved imports (filled in separately after parsing) *)

type lib_path = {package : string option; path : string}
type resolved_import =
  | Unresolved
  | LibPath of lib_path
  | IDLPath of (string * string) (* filepath * bytes *)
  | ImportedValuePath of string
  | PrimPath (* the built-in prim module *)

(* Identifiers *)

type id = string Source.phrase
(* type id_ref, see below *)
type typ_id = (string, Type.con option) Source.annotated_phrase


(* Types *)

type 'note sort = (Type.obj_sort, 'note) Source.annotated_phrase
type typ_obj_sort = unit sort
type persistence = bool Source.phrase
type obj_sort = persistence sort
type func_sort = Type.func_sort Source.phrase

type mut = mut' Source.phrase
and mut' = Const | Var

and path = (path', Type.typ) Source.annotated_phrase
and path' =
  | IdH  of id
  | DotH of path * id

and async_sort = Type.async_sort
and await_sort = Type.await_sort

type typ = (typ', Type.typ) Source.annotated_phrase
and typ' =
  | PathT of path * typ list                       (* type path *)
  | PrimT of string                                (* primitive *)
  | ObjT of typ_obj_sort * typ_field list          (* object *)
  | ArrayT of mut * typ                            (* array *)
  | OptT of typ                                    (* option *)
  | VariantT of typ_tag list                       (* variant *)
  | TupT of typ_item list                          (* tuple *)
  | FuncT of func_sort * typ_bind list * typ * typ (* function *)
  | AsyncT of async_sort * scope * typ             (* future / computation *)
  | AndT of typ * typ                              (* intersection *)
  | OrT of typ * typ                               (* union *)
  | ParT of typ                                    (* parentheses, used to control function arity only *)
  | NamedT of id * typ                             (* parenthesized single element named "tuple" *)
  | WeakT of typ                                   (* weak reference *)

and scope = typ
and typ_field = typ_field' Source.phrase
and typ_field' =
  | ValF of id * typ * mut
  | TypF of typ_id * typ_bind list * typ

and typ_tag = typ_tag' Source.phrase
and typ_tag' = {tag : id; typ : typ}

and bind_sort = Type.bind_sort Source.phrase
and typ_bind = (typ_bind', Type.con option) Source.annotated_phrase
and typ_bind' = {var : id; sort : bind_sort; bound : typ;}

and typ_item = id option * typ


(* Literals *)

type lit =
  | NullLit
  | BoolLit of bool
  | NatLit of Numerics.Nat.t
  | Nat8Lit of Numerics.Nat8.t
  | Nat16Lit of Numerics.Nat16.t
  | Nat32Lit of Numerics.Nat32.t
  | Nat64Lit of Numerics.Nat64.t
  | IntLit of Numerics.Int.t
  | Int8Lit of Numerics.Int_8.t
  | Int16Lit of Numerics.Int_16.t
  | Int32Lit of Numerics.Int_32.t
  | Int64Lit of Numerics.Int_64.t
  | FloatLit of Numerics.Float.t
  | CharLit of Value.unicode
  | TextLit of string
  | BlobLit of string
  | PreLit of string * Type.prim


(* Patterns *)

type pat = (pat', Type.typ) Source.annotated_phrase
and pat' =
  | WildP                                      (* wildcard *)
  | VarP of id                                 (* variable *)
  | LitP of lit ref                            (* literal *)
  | SignP of unop * lit ref                    (* signed literal *)
  | TupP of pat list                           (* tuple *)
  | ObjP of pat_field list                     (* object *)
  | OptP of pat                                (* option *)
  | TagP of id * pat                           (* tagged variant *)
  | AltP of pat * pat                          (* disjunctive *)
  | AnnotP of pat * typ                        (* type annotation *)
  | ParP of pat                                (* parenthesis *)
(*
  | AsP of pat * pat                           (* conjunctive *)
*)

and pat_field = pat_field' Source.phrase
and pat_field' =
  | ValPF of id * pat
  | TypPF of typ_id

let pf_id pf = match pf.Source.it with
  | ValPF(id, _) -> id
  | TypPF(id) -> Source.{ it = id.it; at = id.at; note = () }

let pf_pattern pf = match pf.Source.it with
  | ValPF(_, pat) -> Some pat
  | TypPF(_) -> None

(* Expressions *)

type vis = vis' Source.phrase
and vis' =
  | Public of string option
  | Private
  | System

let is_public vis = match vis.Source.it with Public _ -> true | _ -> false
let is_private vis = match vis.Source.it with Private -> true | _ -> false

type stab = stab' Source.phrase
and stab' = Stable | Flexible

type op_typ = Type.typ ref (* For overloaded resolution; initially Type.Pre. *)

type inst = ((bool * typ list) option, Type.typ list) Source.annotated_phrase (* For implicit scope instantiation *)

type sort_pat = (Type.shared_sort * pat) Type.shared Source.phrase

type sugar = bool (* Is the source of a function body a block `<block>`,
                     subject to further desugaring during parse,
                     or the invariant form `= <exp>`.
                     In the final output of the parser, the exp in FuncE is
                     always in its fully desugared form and the
                     value of the sugar field is irrelevant.
                     This flag is used to correctly desugar an actor's
                     public functions as oneway, shared functions *)

type loop_flags = { mutable has_break : bool; mutable has_continue : bool }

let new_loop_flags () : loop_flags = { has_break = false; has_continue = false }

type control = Break | Continue

let auto_s = "<>auto"
let auto_continue_s = "continue <>auto"

let break_label kind (id_opt : id option) =
  match kind, id_opt with
  | Break, None -> auto_s
  | Continue, None -> auto_continue_s
  | _, Some {Source.it; _} -> it


type id_ref = (string, mut' * exp option) Source.annotated_phrase
and hole_sort = Named of string | Anon of int
and exp = (exp', typ_note) Source.annotated_phrase
and exp' =
  | HoleE of hole_sort * exp ref
  | PrimE of string                            (* primitive *)
  | VarE of id_ref                             (* variable *)
  | LitE of lit ref                            (* literal *)
  | ActorUrlE of exp                           (* actor reference *)
  | UnE of op_typ * unop * exp                 (* unary operator *)
  | BinE of op_typ * exp * binop * exp         (* binary operator *)
  | RelE of op_typ * exp * relop * exp         (* relational operator *)
  | ShowE of (op_typ * exp)                    (* debug show operator *)
  | ToCandidE of exp list                      (* to_candid operator *)
  | FromCandidE of exp                         (* from_candid operator *)
  | TupE of exp list                           (* tuple *)
  | ProjE of exp * int                         (* tuple projection *)
  | OptE of exp                                (* option injection *)
  | DoOptE of exp                              (* option monad *)
  | BangE of exp                               (* scoped option projection *)
  | ObjBlockE of exp option * obj_sort * (id option * typ option) * dec_field list  (* object block *)
  | ObjE of exp list * exp_field list          (* record literal/extension *)
  | TagE of id * exp                           (* variant *)
  | DotE of exp * id * contextual_dot_note     (* object projection *)
  | AssignE of exp * exp                       (* assignment *)
  | ArrayE of mut * exp list                   (* array *)
  | IdxE of exp * exp                          (* array indexing *)
  | FuncE of string * sort_pat * typ_bind list * pat * typ option * sugar * exp  (* function *)
  | CallE of exp option * exp * inst * arg_exp     (* function call *)
  | BlockE of dec list                         (* block (with type after avoidance) *)
  | NotE of exp                                (* negation *)
  | AndE of exp * exp                          (* conjunction *)
  | OrE of exp * exp                           (* disjunction *)
  | IfE of exp * exp * exp                     (* conditional *)
  | SwitchE of exp * case list                 (* switch *)
  | WhileE of exp * exp * loop_flags       (* while-do loop *)
  | LoopE of exp * exp option * loop_flags (* do-while loop *)
  | ForE of pat * exp * exp * loop_flags   (* iteration *)
  | LabelE of id * typ * exp                   (* label *)
  | BreakE of control * id option * exp        (* break *)
  | RetE of exp                                (* return *)
  | DebugE of exp                              (* debugging *)
  | AsyncE of exp option * async_sort * typ_bind * exp (* future / computation *)
  | AwaitE of await_sort * exp                 (* await *)
  | AssertE of assert_kind * exp               (* assertion *)
  | AnnotE of exp * typ                        (* type annotation *)
  | ImportE of (string * resolved_import ref)  (* import statement *)
  | ImplicitLibE of string                     (* implicitly imported library *)
  | ThrowE of exp                              (* throw exception *)
  | TryE of exp * case list * exp option       (* catch exception / finally *)
  | IgnoreE of exp                             (* ignore *)
(*
  | AtomE of string                            (* atom *)
 *)
and arg_exp = (bool * (exp ref))

and assert_kind =
  | Runtime

and dec_field = dec_field' Source.phrase
and dec_field' = {dec : dec; vis : vis; stab: stab option}

and exp_field = exp_field' Source.phrase
and exp_field' = {mut : mut; id : id; exp : exp}

and case = case' Source.phrase
and case' = {pat : pat; exp : exp}

(* When `Some`, this holds the expression that produces the function to apply to the receiver.
   eg. when `x.f(args...)` desugars to `M.f(x, args...)` the note will hold `M.f` *)
and contextual_dot_note = exp option ref

(* Declarations *)

and dec = (dec', typ_note) Source.annotated_phrase
and dec' =
  | ExpD of exp                                (* plain unit expression *)
  | LetD of pat * exp * exp option             (* immutable, with an optional fail block *)
  | VarD of id * exp                           (* mutable *)
  | TypD of typ_id * typ_bind list * typ       (* type *)
  | ClassD of                                  (* class *)
      exp option * sort_pat * obj_sort * typ_id * typ_bind list * pat * typ option * id * dec_field list
  | MixinD of pat * dec_field list             (* mixin *)
  | IncludeD of id * exp * include_note (* mixin include *)
and include_note' = { imports : import list; pat : pat; decs : dec_field list }
and include_note = include_note' option ref

and import = (import', Type.typ) Source.annotated_phrase
and import' = pat * string * resolved_import ref

(* Program (pre unit detection) *)

type prog_note = { filename : string; trivia : Trivia.triv_table }
type prog = (prog', prog_note) Source.annotated_phrase
and prog' = dec list

(* Signatures (stable variables) *)

type stab_sig = (stab_sig', prog_note) Source.annotated_phrase
and stab_sig' = (dec list * stab_body)      (* type declarations & stable actor fields *)
and stab_body = stab_body' Source.phrase    (* type declarations & stable actor fields *)
and stab_body' =
  | Single of typ_field list
  | PrePost of (req * typ_field) list * typ_field list
and req = bool Source.phrase

(* Compilation units *)

type comp_unit_body = (comp_unit_body', typ_note) Source.annotated_phrase
and comp_unit_body' =
 | ProgU of dec list                         (* main programs *)
 | ActorU of persistence * exp option * id option * dec_field list      (* main IC actor *)
 | ModuleU of id option * dec_field list     (* module library *)
 | ActorClassU of                            (* IC actor class, main or library *)
     persistence * exp option * sort_pat * typ_id * typ_bind list * pat * typ option * id * dec_field list
 | MixinU of pat * dec_field list            (* Mixins *)

type comp_unit = (comp_unit', prog_note) Source.annotated_phrase
and comp_unit' = {
  imports : import list;
  body : comp_unit_body;
  }

type lib = comp_unit


(* Helpers *)

let (@@) = Source.(@@)
let (@~) it at = Source.annotate (Const, None) it at
let (@?) it at = Source.annotate empty_typ_note it at
let (@!) it at = Source.annotate Type.Pre it at
let (@=) it at = Source.annotate None it at

(* NB: This function is currently unused *)
let string_of_lit = function
  | BoolLit false -> "false"
  | BoolLit true  -> "true"
  | IntLit n
  | NatLit n      -> Numerics.Int.to_pretty_string n
  | Int8Lit n     -> Numerics.Int_8.to_pretty_string n
  | Int16Lit n    -> Numerics.Int_16.to_pretty_string n
  | Int32Lit n    -> Numerics.Int_32.to_pretty_string n
  | Int64Lit n    -> Numerics.Int_64.to_pretty_string n
  | Nat8Lit n     -> Numerics.Nat8.to_pretty_string n
  | Nat16Lit n    -> Numerics.Nat16.to_pretty_string n
  | Nat32Lit n    -> Numerics.Nat32.to_pretty_string n
  | Nat64Lit n    -> Numerics.Nat64.to_pretty_string n
  | CharLit c     -> string_of_int c
  | NullLit       -> "null"
  | TextLit t     -> t
  | BlobLit b     -> b
  | FloatLit f    -> Numerics.Float.to_pretty_string f
  | PreLit _      -> assert false

(** Used for debugging *)
let string_of_resolved_import = function
  | LibPath {path; package } -> Printf.sprintf "LibPath {path = %s, package = %s}" path (match package with | Some p -> p | None -> "None")
  | IDLPath (path, _) -> Printf.sprintf "IDLPath (%s, _)" path
  | ImportedValuePath path -> Printf.sprintf "ImportedValuePath %s" path
  | PrimPath -> "PrimPath"
  | Unresolved -> "Unresolved"

(* Miscellaneous *)
(* TODO: none of what follows should probably be in this file *)

open Source


(* Identifiers *)

let anon_id sort at = "@anon-" ^ sort ^ "-" ^ string_of_pos at.left
let is_anon_id id = Lib.String.chop_prefix "@anon-" id.it <> None

let is_privileged name =
  String.length name > 0 && name.[0] = '@'

let is_underscored name =
  String.length name > 0 && name.[0] = '_'

let is_scope name =
  String.length name > 0 && name.[0] = '$'

(* Types & Scopes *)

let arity t =
  match t.Source.it with
  | TupT ts -> List.length ts
  | _ -> 1

let is_any t =
  match t.it with
  | PrimT "Any" -> true
  | _ -> false

let scopeT at =
  PathT (IdH {it = Type.default_scope_var; at; note = ()} @! at, []) @! at


(* Expressions *)

let asyncE sort tbs e =
  AsyncE (None, sort, tbs, e) @? e.at

let ignore_asyncE tbs e =
  IgnoreE (
    AnnotE (AsyncE (None, Type.Fut, tbs, e) @? e.at,
      AsyncT (Type.Fut, scopeT e.at, TupT [] @! e.at) @! e.at) @? e.at ) @? e.at

let is_asyncE e =
  match e.it with
  | AsyncE _ -> true
  | _ -> false

let is_ignore_asyncE e =
  match e.it with
  | IgnoreE
      {it = AnnotE ({it = AsyncE (None, Type.Fut, _, _); _},
        {it = AsyncT (Type.Fut, _, {it = TupT []; _}); _}); _} ->
    true
  | _ -> false

let contextual_dot_args e1 e2 dot_note =
  let module T = Mo_types.Type in
  let arity = match dot_note.note.note_typ with
    | T.Func(_, _, _, args, _) -> List.length args
    | _ -> raise (Invalid_argument "non-function type in contextual dot note") in
  let effect eff =
    match (e1.note.note_eff, eff) with
    | T.Triv, T.Triv -> T.Triv
    | _, _ -> T.Await
  in
  let args = match e2 with
    | { it = TupE []; at; note = { note_eff;_ } } ->
       { it = e1.it; at; note = { note_eff = effect note_eff; note_typ = e1.note.note_typ } }
    | { it = TupE exps; at; note = { note_eff; note_typ = T.Tup ts } } when arity <> 2 ->
       { it = TupE (e1::exps); at; note = { note_eff = effect note_eff; note_typ = T.Tup (e1.note.note_typ::ts) } }
    | { at; note = { note_eff; _ }; _ } ->
       { it = TupE ([e1; e2]); at; note = { note_eff = effect note_eff; note_typ = T.Tup ([e1.note.note_typ; e2.note.note_typ]) } }
  in args
