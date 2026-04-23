// and-pattern in a context that needs type inference (no scrutinee
// type to propagate to `check_pat`) — hits the dedicated M0261
// "cannot infer and-pattern" code.
func f(x and y) : Nat = x;
ignore f 3
