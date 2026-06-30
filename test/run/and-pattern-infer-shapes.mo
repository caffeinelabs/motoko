//MOC-FLAG -A=M0145
// AndP in inference position with structurally-rich explicit legs.
// Each function parameter is an AndP in `infer_pat` context (no outer
// type annotation on the argument), so `is_explicit_pat` is consulted
// to decide which leg drives inference.  Each case hits a different
// OCaml arm of `is_explicit_pat` / `is_explicit_pat_field` that were
// previously dark (only `AnnotP` was covered).

// LitP arm — BoolLit is the only explicit literal; `true` drives
// inference for `y`, so the parameter type is inferred as Bool.
func fLit(true and _y) : Bool = true;
assert (fLit true == true);

// OptP arm — explicit iff inner is explicit (inner is AnnotP here).
// The OptP leg drives inference for `b`; b : ?Nat.
func fOpt(?(a : Nat) and _b) : ?Nat = ?a;
assert (fOpt (?7) == ?7);

// TagP arm — explicit iff payload is explicit.
func fTag(#foo (a : Nat) and _b) : {#foo : Nat} = #foo a;
assert (fTag (#foo 3) == #foo 3);

// TupP arm — explicit iff ALL elements are explicit.
// Both elements carry annotations, so the TupP drives inference.
func fTup((a : Nat, b : Text) and _c) : (Nat, Text) = (a, b);
assert (fTup (5, "hi") == (5, "hi"));

// ObjP arm + ValPF arm of is_explicit_pat_field:
// Both value fields are annotated, so ObjP is explicit.
func fObjVal({ x = (a : Nat); y = (_b : Text) } and _c) : { x : Nat; y : Text } =
  { x = a; y = "z" };
assert (fObjVal { x = 1; y = "z" } == { x = 1; y = "z" });

// Nested AndP arm: the left leg of the outer AndP is itself an AndP,
// so `is_explicit_pat (AndP (p1, p2))` is evaluated recursively.
// Inner left leg is AnnotP (explicit) => inner AndP is explicit
// => outer AndP has an explicit left leg.
func fAndNest(((a : Nat) and _b) and _c) : Nat = a;
assert (fAndNest 4 == 4);
