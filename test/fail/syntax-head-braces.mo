// The branches (or body) of an unparenthesized extended head must be blocks (M0272).
func f(x : Nat) : Bool = x > 0;
let y = if f(1) 1 else 2;
