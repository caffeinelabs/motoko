(* Structural deduplication of zero-arity type constructors in a stable
   signature.

   The stable-sig printer emits a [type Name = body] declaration for each
   reachable type constructor. In long migration chains the same shape
   (e.g. an [Event] variant) is repeatedly redeclared under different
   nominal names, ballooning the [icp:private motoko:stable-types] custom
   section past the 1 MiB IC limit.

   [build_subst s] groups every reachable zero-arity, parameter-less,
   non-bare-alias constructor by [Typ_hash.typ_hash] of its body and picks
   one canonical representative per group. It returns a [con -> con option]
   substitution that maps each non-rep cons to its rep; non-deduped cons
   map to [None].

   The substitution is intended to be applied at render time by
   [Type.string_of_stab_sig_with_subst]. The Motoko type graph
   ([Cons.kind]) is left untouched. *)

val build_subst : Type.stab_sig -> (Type.con -> Type.con option)
