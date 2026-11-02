//MOC-FLAG -A=M0145
// AndP in inference position, accepted when a leg is explicit enough to drive
// inference; cases walk each is_explicit_pat / is_explicit_pat_field arm.

func fL((x : Nat) and y) : Nat = x + y;
assert (fL 3 == 6);

func fR(x and (y : Nat)) : Nat = x + y;
assert (fR 4 == 8);

func fB((x : Nat) and (y : Nat)) : Nat = x + y;
assert (fB 5 == 10);

let ((a : Nat) and b) = 7;
assert (a + b == 14);

let ((p : Nat) and q, (r : Nat) and s) = (11, 13);
assert (p + q + r + s == 48);

func fLit(true and _y) : Bool = true;
assert (fLit true == true);

func fOpt(?(a : Nat) and _b) : ?Nat = ?a;
assert (fOpt (?7) == ?7);

func fTag(#foo (a : Nat) and _b) : {#foo : Nat} = #foo a;
assert (fTag (#foo 3) == #foo 3);

func fTup((a : Nat, b : Text) and _c) : (Nat, Text) = (a, b);
assert (fTup (5, "hi") == (5, "hi"));

func fObjVal({ x = (a : Nat); y = (_b : Text) } and _c) : { x : Nat; y : Text } =
  { x = a; y = "z" };
assert (fObjVal { x = 1; y = "z" } == { x = 1; y = "z" });

func fAndNest(((a : Nat) and _b) and _c) : Nat = a;
assert (fAndNest 4 == 4);
