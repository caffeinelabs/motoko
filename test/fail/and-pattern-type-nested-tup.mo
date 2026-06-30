// M0260 where the duplicate type id is nested inside a tuple pattern —
// exercises the `TupP` arm of `find_typ_id_at` in typing.ml.
// `find_typ_id_at` enters TupP, searches the first element (ObjP with TypPF T),
// and finds T there.
let ({ type T }, ()) and ({ type T }, ()) : (module { type T = Nat }, ()) =
  (module { public type T = Nat }, ());
