// The compact defaulted form `label x = <default> <body>` (no type annotation)
// does not typecheck with a non-bottom default: without an annotation the label
// type is (), so the default `false : Bool` — and the `break hit true` value —
// are checked against (), not inferred.  An annotation fixes it
// (`label hit : Bool = false ...`); see #6163 and test/run/label-default.mo.
func d(xs : [Nat], target : Nat) : Bool {
  label hit = false
    for (x in xs.vals()) { if (x == target) break hit true }
};
