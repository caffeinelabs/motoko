(* Compress a stable signature for printing.

   Two passes are applied before serialisation:
   1. Structural deduplication of zero-arity, parameter-less [Def] cons
      whose bodies hash equal under [Typ_hash.typ_hash].
   2. Anchored-intersection mining over plain [Object] records: for each
      reachable field, the largest itemset shared by all records
      containing it is a candidate base. Bases that pay off (per a
      simple cost model) are minted as fresh synthetic [Cons.t]s and
      every covered record is rewritten as
      [Base__N and Base__M and { delta }].

   Stability checks compare types structurally, so the rewrite is
   observable only as a smaller [.most] file. *)

val string_of : Type.stab_sig -> string
