// M10 axiom: destruct ⇒ construct for tuples carrying a top-level
// existential. The round-trip via a block (destructure, then
// re-construct from the destructured parts) lands back at the
// original existential type, with the witness preserved across the
// pack/unpack cycle.
//
// (Same shape as the variant-arm axiom 3 — `case (#tag p) #tag p` —
// but with a tuple's `let`-destructure standing in for the `case`.)

type Add = type X in (Int, X, X -> Int);

func natToInt(n : Nat) : Int = n;
func boolToInt(b : Bool) : Int = if b 1 else 0;

let trip1 : Add = (42, 7, natToInt);
let trip2 : Add = (-1, true, boolToInt);

// Destruct + re-construct — block annotated as Add re-packs the witness.
let r1 : Add = do { let (i, h, f) = trip1; (i, h, f) };
let r2 : Add = do { let (i, h, f) = trip2; (i, h, f) };

// Apply `f` to `h` on both sides to verify the witnesses still match.
let i1a = do { let (i, h, f) = trip1; (i, f h) };
let i1b = do { let (i, h, f) = r1;    (i, f h) };
assert i1a == i1b;
assert i1a == (42, 7);

let i2a = do { let (i, h, f) = trip2; (i, f h) };
let i2b = do { let (i, h, f) = r2;    (i, f h) };
assert i2a == i2b;
assert i2a == (-1, 1);
