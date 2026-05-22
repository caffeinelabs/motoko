// AndP in inference position: previously rejected wholesale with
// M0261; now accepted when at least one leg carries enough type
// information to drive inference (`is_explicit_pat` true on a leg).

// Left leg annotated — pat1 drives inference, pat2 checked against
// the inferred Nat.
func fL((x : Nat) and y) : Nat = x + y;
assert (fL 3 == 6);

// Right leg annotated — symmetric.
func fR(x and (y : Nat)) : Nat = x + y;
assert (fR 4 == 8);

// Both legs annotated: AndP's type is the glb of the two. Here both
// are Nat, so Nat.
func fB((x : Nat) and (y : Nat)) : Nat = x + y;
assert (fB 5 == 10);

// let-context, left-annotated: `a` typed as Nat, `b` checked against Nat.
let ((a : Nat) and b) = 7;
assert (a + b == 14);

// TupP surrounding AndP: each leg gets to drive its own inference.
let ((p : Nat) and q, (r : Nat) and s) = (11, 13);
assert (p + q + r + s == 48);
