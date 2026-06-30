// M0260 where the duplicate type id in the second leg is nested inside
// an inner AndP — exercises the `AndP` arm of `find_typ_id_at` in typing.ml.
// The inner `x and { type T }` is the second leg of the outer and-pattern;
// `find_typ_id_at` recurses through the inner AndP's left (VarP x, no T),
// then finds T in the inner AndP's right (ObjP [{TypPF T}]).
let { type T } and (x and { type T }) : module { type T = Nat } =
  module { public type T = Nat };
