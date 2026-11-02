// Legs with incompatible types: `(x : Nat)` and `(y : Text)` cannot
// both accept a `Nat 5` — expect a clean subtype error naming the
// offending leg, not a confused message.
let (x : Nat) and (y : Text) = 5;
ignore x;
ignore y
