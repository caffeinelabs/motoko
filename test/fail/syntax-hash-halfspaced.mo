// `#` glued to an identifier introduces a variant, so the half-spaced concatenation `a #b` no longer parses;
// both `a # b` and `a#b` remain concatenation.
let a = "a"; let b = "b";
let ok1 = a # b;
let ok2 = a#b;
let bad = a #b;
