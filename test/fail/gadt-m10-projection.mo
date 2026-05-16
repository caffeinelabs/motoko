// M10: bare projection on existential-bearing types is rejected
// (Cardelli's `open` discipline — destructuring is the only way
// to bring the witness into a controlled scope).

type Tup = type X in (X, X -> Text);
type Rec = type X in { value : X; render : X -> Text };

let tup : Tup = (5 : Nat, func (n : Nat) : Text = debug_show n);
let rec_ : Rec = { value = 5 : Nat; render = func (n : Nat) : Text = debug_show n };

// Bare tuple projection — should be rejected.
let _x = tup.0;

// Bare record projection — should be rejected.
let _v = rec_.value;
