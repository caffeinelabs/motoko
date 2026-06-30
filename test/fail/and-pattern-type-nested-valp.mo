// M0260 where the duplicate type id is nested inside a value-field
// pattern — exercises the `ValPF` arm of `find_typ_id_at` in typing.ml.
// `find_typ_id_at` enters the ObjP arm, encounters ValPF("inner", ...),
// and recurses into the nested ObjP [{TypPF T}] to find T.
let { inner = { type T } } and { inner = { type T } } =
  { inner = module { public type T = Nat } };
