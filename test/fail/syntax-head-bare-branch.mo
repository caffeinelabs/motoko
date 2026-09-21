// An `if` whose condition is not parenthesized takes braced branches (M0275): `if c { 1 } else { 2 }`.
// The v1 form with bare branches keeps working with a parenthesized condition: `if (c) 1 else 2`.
let c = true;
let a = if c 1 else 2;
