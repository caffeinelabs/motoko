// M10 records: existential bindings at the top of a record type.
// `type Rec = type X in { value : X; render : X -> Text }` declares
// a record whose `value` field has some hidden X and whose `render`
// field can stringify that X. Witness packed at construction;
// destructuring re-introduces X as an opaque skolem.

type Rec = type X in { value : X; render : X -> Text };

func natToText(n : Nat) : Text = debug_show n;
func boolToText(b : Bool) : Text = debug_show b;

// Construction unifies the existential: X = Nat for r1, X = Bool for r2.
let r1 : Rec = { value = 5; render = natToText };
let r2 : Rec = { value = true; render = boolToText };

// Destructuring re-introduces X as a fresh skolem; we can only use
// `value` by passing it to `render`.
let { value = v1; render = f1 } = r1;
assert f1 v1 == "5";

let { value = v2; render = f2 } = r2;
assert f2 v2 == "true";
