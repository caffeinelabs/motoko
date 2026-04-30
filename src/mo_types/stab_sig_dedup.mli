(* Structural deduplication of zero-arity type constructors in a stable
   signature.

   The stable-sig printer emits a [type Name = body] declaration for each
   reachable type constructor. In long migration chains the same shape
   (e.g. an [Event] variant) is repeatedly redeclared under different
   nominal names, ballooning the [icp:private motoko:stable-types] custom
   section past the 1 MiB IC limit.

   [dedup s] returns a stable signature in which every group of
   structurally-equivalent zero-arity, parameter-less constructors
   collapses to a single canonical representative. Stability checks
   compare signatures structurally (see [Mo_frontend.Stability]), so
   the rewrite is observable only as a smaller [.most] file.

   Constructors with type parameters or in [Abs] kinds are left
   untouched. *)

val dedup : Type.stab_sig -> Type.stab_sig
