// After an unparenthesized head, `else if` chains only into braced branches:
// a legacy bare-branch `if` cannot ride on the chain.
func f(x : Nat) : Bool = x > 0;
let y = if f(1) { 1 } else if (true) 2 else 3;
