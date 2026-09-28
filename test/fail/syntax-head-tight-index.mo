// An unspaced `[` (or `(`) directly after the condition extends it: `c[1]` indexes `c`, so the branches must be blocks (M0275).
// The legacy reading `if c` with the bare branch `[1]` needs a space: `if c [1] else []`.
let c = true;
let a : [Nat] = if c[1] else [];
