(** A zipper for navigating the Motoko AST with parent context.

    NOTE: This module was AI-generated

    This module provides a lightweight, read-only zipper over the heterogeneous
    Motoko AST. Rather than encoding full one-hole contexts (which would require
    hundreds of constructors), we use a simple "crumb trail" approach: each
    crumb records the parent node and the child's position within it.

    This is sufficient for read-only queries like "is this node the top-level
    pattern of a let-binding?" without the complexity of reconstructing modified
    trees. *)

open Syntax

(** {1 Unified AST Node Type} *)

(** A single type that can hold any AST node we care about navigating through. *)
type node =
  | ExpNode of exp
  | DecNode of dec
  | PatNode of pat
  | TypNode of typ
  | DecFieldNode of dec_field
  | ExpFieldNode of exp_field
  | CaseNode of case
  | TypBindNode of typ_bind
  | PatFieldNode of pat_field
  | ProgNode of prog
  | CompUnitNode of comp_unit

(** {1 Crumbs and Zipper} *)

(** A breadcrumb recording where we came from: the parent node and
    which child position (0-based) led to the current focus. *)
type crumb = {
  parent : node;
  child_index : int;
}

(** The zipper: a focused node together with a trail of breadcrumbs
    leading back to the root. The head of [context] is the immediate parent. *)
type t = {
  focus : node;
  context : crumb list;
}

(** {1 Construction} *)

let empty : t =
  let open Source in
  let note = { filename = ""; trivia = Trivia.empty_triv_table } in
  { focus = ProgNode (annotate note [] no_region); context = [] }

let of_prog (p : prog) : t =
  { focus = ProgNode p; context = [] }

let of_comp_unit (cu : comp_unit) : t =
  { focus = CompUnitNode cu; context = [] }

(** {1 Navigation Helpers} *)

(** Descend into a child node, pushing the current focus onto the context. *)
let descend (z : t) (child_index : int) (child : node) : t =
  { focus = child;
    context = { parent = z.focus; child_index } :: z.context }

(** Move up to the parent, if any. Returns [None] at the root. *)
let up (z : t) : crumb option =
  match z.context with
  | [] -> None
  | crumb :: _ -> Some crumb

(** The full trail of ancestors from immediate parent to root. *)
let ancestors (z : t) : crumb list = z.context

(** The immediate parent node, if any. *)
let parent (z : t) : node option =
  match z.context with
  | [] -> None
  | { parent; _ } :: _ -> Some parent

(** {1 Child Extraction}

    These functions enumerate the interesting children of each node type,
    returning them as [(index, node)] pairs. We only descend into children
    that are themselves AST nodes we track in {!node}. *)

let exp_children (e : exp) : (int * node) list =
  match e.Source.it with
  | PrimE _ | VarE _ | LitE _ | ImportE _ | ImplicitLibE _ -> []
  | HoleE (_, e_ref) -> [(0, ExpNode !e_ref)]
  | ActorUrlE e1 -> [(0, ExpNode e1)]
  | UnE (_, _, e1) -> [(0, ExpNode e1)]
  | ShowE (_, e1) -> [(0, ExpNode e1)]
  | FromCandidE e1 -> [(0, ExpNode e1)]
  | OptE e1 -> [(0, ExpNode e1)]
  | DoOptE e1 -> [(0, ExpNode e1)]
  | BangE e1 -> [(0, ExpNode e1)]
  | TagE (_, e1) -> [(0, ExpNode e1)]
  | ProjE (e1, _) -> [(0, ExpNode e1)]
  | NotE e1 -> [(0, ExpNode e1)]
  | RetE e1 -> [(0, ExpNode e1)]
  | DebugE e1 -> [(0, ExpNode e1)]
  | IgnoreE e1 -> [(0, ExpNode e1)]
  | ThrowE e1 -> [(0, ExpNode e1)]
  | AwaitE (_, e1) -> [(0, ExpNode e1)]
  | AssertE (_, e1) -> [(0, ExpNode e1)]
  | BinE (_, e1, _, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | RelE (_, e1, _, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | AndE (e1, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | OrE (e1, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | AssignE (e1, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | IdxE (e1, e2) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | WhileE (e1, e2, _) -> [(0, ExpNode e1); (1, ExpNode e2)]
  | ForE (p, e1, e2, _) -> [(0, PatNode p); (1, ExpNode e1); (2, ExpNode e2)]
  | IfE (e1, e2, e3) -> [(0, ExpNode e1); (1, ExpNode e2); (2, ExpNode e3)]
  | LabelE (_, _, e1) -> [(0, ExpNode e1)]
  | BreakE (_, _, e1) -> [(0, ExpNode e1)]
  | AnnotE (e1, t) -> [(0, ExpNode e1); (1, TypNode t)]
  | TupE es -> List.mapi (fun i e -> (i, ExpNode e)) es
  | ArrayE (_, es) -> List.mapi (fun i e -> (i, ExpNode e)) es
  | ToCandidE es -> List.mapi (fun i e -> (i, ExpNode e)) es
  | BlockE ds -> List.mapi (fun i d -> (i, DecNode d)) ds
  | SwitchE (e1, cs) ->
    (0, ExpNode e1) :: List.mapi (fun i c -> (i + 1, CaseNode c)) cs
  | TryE (e1, cs, e_opt) ->
    let base = [(0, ExpNode e1)] in
    let cases = List.mapi (fun i c -> (i + 1, CaseNode c)) cs in
    let finally = match e_opt with
      | Some e2 -> [(List.length cs + 1, ExpNode e2)]
      | None -> []
    in
    base @ cases @ finally
  | LoopE (e1, e_opt, _) ->
    let base = [(0, ExpNode e1)] in
    (match e_opt with Some e2 -> base @ [(1, ExpNode e2)] | None -> base)
  | ObjBlockE (e_opt, _, _, dfs) ->
    let base = match e_opt with
      | Some e1 -> [(0, ExpNode e1)]
      | None -> []
    in
    let offset = List.length base in
    base @ List.mapi (fun i df -> (i + offset, DecFieldNode df)) dfs
  | ObjE (bases, efs) ->
    let bs = List.mapi (fun i e -> (i, ExpNode e)) bases in
    let offset = List.length bs in
    bs @ List.mapi (fun i ef -> (i + offset, ExpFieldNode ef)) efs
  | DotE (e1, _, _) -> [(0, ExpNode e1)]
  | FuncE (_, _, tbs, p, _, _, e1) ->
    let children = List.mapi (fun i tb -> (i, TypBindNode tb)) tbs in
    let offset = List.length children in
    children @ [(offset, PatNode p); (offset + 1, ExpNode e1)]
  | CallE (e_opt, e1, _, (_, e2_ref)) ->
    let base = match e_opt with
      | Some e0 -> [(0, ExpNode e0); (1, ExpNode e1)]
      | None -> [(0, ExpNode e1)]
    in
    let offset = List.length base in
    base @ [(offset, ExpNode !e2_ref)]
  | AsyncE (e_opt, _, _, e1) ->
    (match e_opt with
     | Some e0 -> [(0, ExpNode e0); (1, ExpNode e1)]
     | None -> [(0, ExpNode e1)])

let dec_children (d : dec) : (int * node) list =
  match d.Source.it with
  | ExpD e -> [(0, ExpNode e)]
  | LetD (p, e, fail_opt) ->
    let base = [(0, PatNode p); (1, ExpNode e)] in
    (match fail_opt with
     | Some e2 -> base @ [(2, ExpNode e2)]
     | None -> base)
  | VarD (_, e) -> [(0, ExpNode e)]
  | TypD (_, tbs, t) ->
    let children = List.mapi (fun i tb -> (i, TypBindNode tb)) tbs in
    children @ [(List.length children, TypNode t)]
  | ClassD (e_opt, _, _, _, tbs, p, _, _, dfs) ->
    let base = match e_opt with
      | Some e -> [(0, ExpNode e)]
      | None -> []
    in
    let offset = List.length base in
    let tb_children = List.mapi (fun i tb -> (i + offset, TypBindNode tb)) tbs in
    let offset2 = offset + List.length tb_children in
    base @ tb_children
    @ [(offset2, PatNode p)]
    @ List.mapi (fun i df -> (i + offset2 + 1, DecFieldNode df)) dfs
  | MixinD (p, dfs) ->
    (0, PatNode p) :: List.mapi (fun i df -> (i + 1, DecFieldNode df)) dfs
  | IncludeD (_, e, _) ->
    [(0, ExpNode e)]

let pat_children (p : pat) : (int * node) list =
  match p.Source.it with
  | WildP | VarP _ | LitP _ | SignP _ -> []
  | TupP ps -> List.mapi (fun i p -> (i, PatNode p)) ps
  | ObjP pfs -> List.mapi (fun i pf -> (i, PatFieldNode pf)) pfs
  | OptP p1 -> [(0, PatNode p1)]
  | TagP (_, p1) -> [(0, PatNode p1)]
  | AltP (p1, p2) -> [(0, PatNode p1); (1, PatNode p2)]
  | AnnotP (p1, t) -> [(0, PatNode p1); (1, TypNode t)]
  | ParP p1 -> [(0, PatNode p1)]

let typ_children (t : typ) : (int * node) list =
  match t.Source.it with
  | PathT _ | PrimT _ -> []
  | ObjT (_, tfs) -> [] (* typ_fields are not tracked as nodes for now *)
  | ArrayT (_, t1) -> [(0, TypNode t1)]
  | OptT t1 -> [(0, TypNode t1)]
  | VariantT _ -> []
  | TupT items -> List.mapi (fun i (_, t) -> (i, TypNode t)) items
  | FuncT (_, tbs, t1, t2) ->
    let children = List.mapi (fun i tb -> (i, TypBindNode tb)) tbs in
    let offset = List.length children in
    children @ [(offset, TypNode t1); (offset + 1, TypNode t2)]
  | AsyncT (_, _, t1) -> [(0, TypNode t1)]
  | AndT (t1, t2) -> [(0, TypNode t1); (1, TypNode t2)]
  | OrT (t1, t2) -> [(0, TypNode t1); (1, TypNode t2)]
  | ParT t1 -> [(0, TypNode t1)]
  | NamedT (_, t1) -> [(0, TypNode t1)]
  | WeakT t1 -> [(0, TypNode t1)]

let dec_field_children (df : dec_field) : (int * node) list =
  [(0, DecNode df.Source.it.dec)]

let exp_field_children (ef : exp_field) : (int * node) list =
  [(0, ExpNode ef.Source.it.exp)]

let case_children (c : case) : (int * node) list =
  [(0, PatNode c.Source.it.pat); (1, ExpNode c.Source.it.exp)]

let typ_bind_children (_tb : typ_bind) : (int * node) list =
  [(0, TypNode _tb.Source.it.bound)]

let pat_field_children (pf : pat_field) : (int * node) list =
  match pf.Source.it with
  | ValPF (_, p) -> [(0, PatNode p)]
  | TypPF _ -> []

(** Return all children of a node as [(index, child_node)] pairs. *)
let children (n : node) : (int * node) list =
  match n with
  | ExpNode e -> exp_children e
  | DecNode d -> dec_children d
  | PatNode p -> pat_children p
  | TypNode t -> typ_children t
  | DecFieldNode df -> dec_field_children df
  | ExpFieldNode ef -> exp_field_children ef
  | CaseNode c -> case_children c
  | TypBindNode tb -> typ_bind_children tb
  | PatFieldNode pf -> pat_field_children pf
  | ProgNode p -> List.mapi (fun i d -> (i, DecNode d)) p.Source.it
  | CompUnitNode cu ->
    let imports = cu.Source.it.imports in
    (* We skip imports for now and go straight to the body *)
    let body = cu.Source.it.body in
    let body_children = match body.Source.it with
      | ProgU ds -> List.mapi (fun i d -> (i, DecNode d)) ds
      | ActorU (_, e_opt, _, dfs) ->
        let base = match e_opt with
          | Some e -> [(0, ExpNode e)]
          | None -> []
        in
        let offset = List.length base in
        base @ List.mapi (fun i df -> (i + offset, DecFieldNode df)) dfs
      | ModuleU (_, dfs) ->
        List.mapi (fun i df -> (i, DecFieldNode df)) dfs
      | ActorClassU (_, e_opt, _, _, tbs, p, _, _, dfs) ->
        let base = match e_opt with
          | Some e -> [(0, ExpNode e)]
          | None -> []
        in
        let offset = List.length base in
        let tb_children = List.mapi (fun i tb -> (i + offset, TypBindNode tb)) tbs in
        let offset2 = offset + List.length tb_children in
        base @ tb_children
        @ [(offset2, PatNode p)]
        @ List.mapi (fun i df -> (i + offset2 + 1, DecFieldNode df)) dfs
      | MixinU (p, dfs) ->
        (0, PatNode p) :: List.mapi (fun i df -> (i + 1, DecFieldNode df)) dfs
    in
    let import_offset = List.length imports in
    (* We could include imports here if needed *)
    List.map (fun (i, n) -> (i + import_offset, n)) body_children

(** {1 Traversal}

    Depth-first traversal of the AST, calling a visitor function at each node
    with the full zipper context available. *)

(** Visit every node in the subtree rooted at [z], calling [f] on each.
    Traversal is depth-first, pre-order. *)
let rec traverse (f : t -> unit) (z : t) : unit =
  f z;
  let child_nodes = children z.focus in
  List.iter (fun (idx, child) ->
    traverse f (descend z idx child)
  ) child_nodes

(** Like {!traverse} but [f] can return [false] to prune the subtree
    (skip children of the current node). *)
let rec traverse_prune (f : t -> bool) (z : t) : unit =
  if f z then begin
    let child_nodes = children z.focus in
    List.iter (fun (idx, child) ->
      traverse_prune f (descend z idx child)
    ) child_nodes
  end

(** Collect all nodes (with context) matching a predicate. *)
let collect (pred : t -> bool) (z : t) : t list =
  let acc = ref [] in
  traverse (fun z -> if pred z then acc := z :: !acc) z;
  List.rev !acc

(** {1 Query Helpers} *)

(** Extract the source region from a node. *)
let region_of_node (n : node) : Source.region =
  match n with
  | ExpNode e -> e.Source.at
  | DecNode d -> d.Source.at
  | PatNode p -> p.Source.at
  | TypNode t -> t.Source.at
  | DecFieldNode df -> df.Source.at
  | ExpFieldNode ef -> ef.Source.at
  | CaseNode c -> c.Source.at
  | TypBindNode tb -> tb.Source.at
  | PatFieldNode pf -> pf.Source.at
  | ProgNode p -> p.Source.at
  | CompUnitNode cu -> cu.Source.at

(** The source region of the focused node. *)
let region (z : t) : Source.region =
  region_of_node z.focus

(** {1 Region Utilities} *)

(** [pos_leq a b] is true when position [a] is at or before position [b]. *)
let pos_leq (a : Source.pos) (b : Source.pos) : bool =
  Source.Pos_ord.compare a b <= 0

(** [encloses outer inner] is true when region [outer] fully contains [inner],
    i.e. outer.left <= inner.left and inner.right <= outer.right. *)
let encloses (outer : Source.region) (inner : Source.region) : bool =
  pos_leq outer.Source.left inner.Source.left
  && pos_leq inner.Source.right outer.Source.right

(** Focus the zipper on the narrowest (deepest) node whose source region
    fully encloses [target]. Traversal is depth-first: we descend into
    children that enclose the target, and stop when no child does, leaving
    us at the tightest enclosing node.

    Returns [None] if not even the root encloses the target. *)
let focus_on_region (target : Source.region) (z : t) : t option =
  let rec go z =
    (* Try to find a child whose region encloses the target *)
    let child_zippers =
      children z.focus
      |> List.filter_map (fun (idx, child) ->
           let child_region = region_of_node child in
           if encloses child_region target
           then Some (descend z idx child)
           else None)
    in
    match child_zippers with
    | [] ->
      (* No child encloses the target; the current focus is the narrowest *)
      Some z
    | first :: rest ->
      (* Among children that enclose the target, pick the one with the
         tightest (smallest) region. In a well-formed AST there should
         typically be at most one, but we handle ties gracefully. *)
      let best =
        List.fold_left (fun best cand ->
          let r_best = region best in
          let r_cand = region cand in
          (* Prefer the candidate if it is strictly enclosed by the current best,
             i.e. it's narrower *)
          if encloses r_best r_cand then cand else best
        ) first rest
      in
      go best
  in
  let root_region = region_of_node z.focus in
  if encloses root_region target then go z else None

(** {1 Specific Queries}

    Higher-level questions you can ask about a node's position in the tree. *)

(** Is the focused pattern the top-level pattern of a [let]-binding?

    This checks whether the current focus is a [PatNode] and its immediate
    parent is a [DecNode] whose [dec'] is [LetD], with the pattern at
    child index 0 (i.e. the binding pattern, not a pattern inside the
    bound expression or fail block). *)
let is_let_bound_pat (z : t) : bool =
  match z.focus, z.context with
  | PatNode _, { parent = DecNode d; child_index = 0 } :: _ ->
    (match d.Source.it with
     | LetD _ -> true
     | _ -> false)
  | _ -> false

(** Is the focused node a [VarP] that is the top-level pattern of a
    [let]-binding? i.e. [let x = ...] *)
let is_let_bound_var (z : t) : bool =
  match z.focus with
  | PatNode p ->
    (match p.Source.it with
     | VarP _ -> is_let_bound_pat z
     | _ -> false)
  | _ -> false

(** Is the focused pattern the top-level pattern of a [for] loop?
    i.e. [for (p in ...) ...] *)
let is_for_bound_pat (z : t) : bool =
  match z.focus, z.context with
  | PatNode _, { parent = ExpNode e; child_index = 0 } :: _ ->
    (match e.Source.it with
     | ForE _ -> true
     | _ -> false)
  | _ -> false

(** Is the focused pattern the parameter pattern of a function? *)
let is_func_param_pat (z : t) : bool =
  match z.focus, z.context with
  | PatNode _, { parent = ExpNode e; child_index } :: _ ->
    (match e.Source.it with
     | FuncE (_, _, tbs, _, _, _, _) ->
       (* The parameter pattern is right after the type bindings *)
       child_index = List.length tbs
     | _ -> false)
  | _ -> false

(** Is the focused expression the body of a [let]-binding? *)
let is_let_bound_exp (z : t) : bool =
  match z.focus, z.context with
  | ExpNode _, { parent = DecNode d; child_index = 1 } :: _ ->
    (match d.Source.it with
     | LetD _ -> true
     | _ -> false)
  | _ -> false

(** Walk up the context looking for an ancestor matching a predicate.
    Returns the first matching crumb, if any. *)
let find_ancestor (pred : node -> bool) (z : t) : crumb option =
  List.find_opt (fun c -> pred c.parent) z.context

(** Is there an ancestor that is a function expression? *)
let is_inside_func (z : t) : bool =
  find_ancestor (fun n ->
    match n with
    | ExpNode e ->
      (match e.Source.it with FuncE _ -> true | _ -> false)
    | _ -> false
  ) z <> None

(** Is there an ancestor that is an async expression? *)
let is_inside_async (z : t) : bool =
  find_ancestor (fun n ->
    match n with
    | ExpNode e ->
      (match e.Source.it with AsyncE _ -> true | _ -> false)
    | _ -> false
  ) z <> None

(** Find all [VarP] nodes that are the top-level pattern of a [let]-binding
    in the given program. *)
let let_bound_vars (root : t) : (t * id) list =
  let results = ref [] in
  traverse (fun z ->
    match z.focus with
    | PatNode p ->
      (match p.Source.it with
       | VarP id when is_let_bound_pat z ->
         results := (z, id) :: !results
       | _ -> ())
    | _ -> ()
  ) root;
  List.rev !results
