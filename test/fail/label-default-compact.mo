// The compact `label x = <default> <body>` (no annotation) takes its type from
// the default — like a `var`/`let` initializer.  A bare sentinel whose type is
// too narrow (here `null : Null`) therefore rejects a wider `break` value.  Fix
// by annotating the default (`label r = (null : ?Nat) …`) or the label
// (`label r : ?Nat = null …`); see #6163 and test/run/label-default.mo.
func firstEven(xs : [Nat]) : ?Nat {
  label r = null
    for (x in xs.vals()) { if (x % 2 == 0) break r (?x) }
};
