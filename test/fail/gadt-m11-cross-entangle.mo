// M11b — "Overly Entangled Black Holes": cross-mixing destructure
// bindings of the same existential-bearing type. Today this is
// silently accepted because both `let`-destructures bind at the
// schema's *shared* skolem cons, so the type system can't tell the
// two black holes apart. Once M11b mints a fresh skolem per
// destructure site, this should be rejected (no subtype relation
// between two unrelated black-hole cons).

type Tup = type X in (X, X -> Text);

func natToText(n : Nat) : Text = debug_show n;
func boolToText(b : Bool) : Text = debug_show b;

let t1 : Tup = (5, natToText);
let t2 : Tup = (true, boolToText);

let (x1, f1) = t1;
let (x2, f2) = t2;

// f1 expects t1's witness, x2 is t2's witness — cross-feed should
// be rejected. Today it type-checks and would runtime-crash.
let _ = f1 x2;
