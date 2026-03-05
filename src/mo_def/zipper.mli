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

(** A breadcrumb recording where we came from: the parent node and
    which child position (0-based) led to the current focus. *)
type crumb = {
  parent : node;
  child_index : int;
}

(** A read-only zipper over the AST *)
type t

(** {1 Construction} *)

(** An empty zipper with no contents *)
val empty : t

(** Create a zipper from a [prog], focusing the root *)
val of_prog : prog -> t

(** Create a zipper from a [comp_unit], focusing the root *)
val of_comp_unit : comp_unit -> t

(** {1 Navigation} *)

(** Move up to the parent, if any. Returns [None] at the root. *)
val up : t -> t option

(** Move to the nth child of the current focus. Throws an exception if out of bounds. *)
val down : t -> int -> t

(** Descend into a child node, pushing the current focus onto the context. *)
val descend : t -> int -> node -> t

(** Focus the zipper on the narrowest (deepest) node whose source region
    fully encloses [target].

    Returns [None] if not even the root encloses the target. *)
val focus_on_region : t -> Source.region -> t option

(** {1 Accessors} *)

(** The current focus *)
val focus : t -> node

(** Return all children of the current focus. *)
val children : t -> node list

(** Return all children of the current focus as [(index, child_node)] pairs. *)
val children' : t -> (int * node) list

(** The immediate parent node, if any. *)
val parent : t -> node option

(** The immediate parent node, if any, as well as the child index we came from *)
val parent' : t -> crumb option

(** Return all ancestors of the current focus as a list of breadcrumbs. *)
val ancestors : t -> crumb list

(** Returns the region of the current focus *)
val region : t -> Source.region

(** {1 Query helpers} *)

(** Extract the source region from a node. *)
val region_of_node : node -> Source.region

(** Return all children of the given node as [(index, child_node)] pairs. *)
val children_of_node : node -> (int * node) list

(** {1 Traversal} *)

(** Visit every node in the subtree rooted at focus, calling [f] on each.
    Traversal is depth-first, pre-order. *)
val traverse : (t -> unit) -> t -> unit

(** Like {!traverse} but [f] can return [false] to prune the subtree
    (skip children of the current node). *)
val traverse_prune : (t -> bool) -> t -> unit

(** {1 Specific Queries} *)

(** Is the focus a variable pattern at the top level of a [let]-binding? *)
val is_let_bound_var : t -> bool
