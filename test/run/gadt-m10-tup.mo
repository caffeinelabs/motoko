// M10 tuples: existential bindings at the top of a type definition.
// `type Tup = type X in (X, X -> Text)` declares a tuple whose first
// element is some hidden X and whose second can render that X. The
// witness type X is packed at construction (`(5, natToText)` → X = Nat)
// and exposed as a fresh skolem on destructuring (the body of the let
// only knows there exists some X, with `x : X` and `f : X -> Text`).

type Tup = type X in (X, X -> Text);

func natToText(n : Nat) : Text = debug_show n;
func boolToText(b : Bool) : Text = debug_show b;

// Construction unifies the existential — X = Nat for t1, X = Bool for t2.
let t1 : Tup = (5, natToText);
let t2 : Tup = (true, boolToText);

// Destructuring re-introduces X as an opaque skolem. We can only use
// `x` by passing it to `f`; the witness type is hidden.
let (x1, f1) = t1;
assert f1 x1 == "5";

let (x2, f2) = t2;
assert f2 x2 == "true";

// The two destructures introduce distinct skolems X1, X2 — you can't
// pass x1 to f2 or x2 to f1; rejection of that would be a separate
// negative test (cf. test/fail/).
