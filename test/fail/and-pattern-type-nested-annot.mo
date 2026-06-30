// M0260 where the duplicate type id is inside an AnnotP in the second leg —
// exercises the `AnnotP (p1, _)` arm of `find_typ_id_at` in typing.ml.
let { type T } and ({ type T } : module { type T = Nat }) =
  module { public type T = Nat };
