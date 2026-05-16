// M10 axiom (records): destruct ⇒ construct round-trip. Mirror of
// gadt-m10-tup-axiom.mo but for record syntax. A `do { let { ... } =
// r; { ... } }` block destructures the existential and re-constructs
// at the same Rec type, with the witness re-packed across the cycle.

type Rec = type X in { value : X; render : X -> Text };

func natToText(n : Nat) : Text = debug_show n;
func boolToText(b : Bool) : Text = debug_show b;

let r1 : Rec = { value = 5; render = natToText };
let r2 : Rec = { value = true; render = boolToText };

let r1_back : Rec = do { let { value = v; render = f } = r1; { value = v; render = f } };
let r2_back : Rec = do { let { value = v; render = f } = r2; { value = v; render = f } };

// Apply render to value on both sides; same result == witness preserved.
let s1a = do { let { value = v; render = f } = r1;  f v };
let s1b = do { let { value = v; render = f } = r1_back; f v };
assert s1a == s1b;
assert s1a == "5";

let s2a = do { let { value = v; render = f } = r2;  f v };
let s2b = do { let { value = v; render = f } = r2_back; f v };
assert s2a == s2b;
assert s2a == "true";
