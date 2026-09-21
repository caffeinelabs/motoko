// `c[1]` is an index into `c`, so this is an unparenthesized condition and the branches must be blocks (M0275).
// To test `c` and produce the array `[1]`, parenthesize the condition: `if (c) [1] else []`.
let c = true;
let a : [Nat] = if c[1] else [];
