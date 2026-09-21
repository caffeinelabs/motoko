// A condition that starts with `(` ends at the matching `)`, as in v1; the rest is then read as a bare branch.
// Parenthesize the whole condition, `if ((a + b) * c > 0) { }`, or drop the leading group.
let a = 1; let b = 2; let c = 3;
if (a + b) * c > 0 { };
