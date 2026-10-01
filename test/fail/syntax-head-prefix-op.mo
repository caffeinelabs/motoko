// In a head, `-`/`+`/`^`/`#` spaced before but not after is prefix-shaped and starts the branch, never a binary operator;
// after an extended head only a block may follow (M0275). Write `inc(1) - 1` or `inc(1)-1` for the subtraction.
func inc(n : Nat) : Nat = n + 1;
if inc(1) -1 > 0 { };
