// Duplicate type id across and-pattern legs (M0260), reached through each
// find_typ_id_at arm: nested AndP, AnnotP, TupP, value-field.
do { let { type T } and (x and { type T }) : module { type T = Nat } = module { public type T = Nat } };
do { let { type T } and ({ type T } : module { type T = Nat }) = module { public type T = Nat } };
do { let ({ type T }, ()) and ({ type T }, ()) : (module { type T = Nat }, ()) = (module { public type T = Nat }, ()) };
do { let { inner = { type T } } and { inner = { type T } } = { inner = module { public type T = Nat } } };
