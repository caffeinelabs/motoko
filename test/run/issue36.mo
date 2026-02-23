//MOC-FLAG -A=M0194
// AST-36: foo.bar.1.zap won't parse

type Z = { zap : Nat };
type B = { bar : (Int, Z) };

let inner : Z = { zap = 42 };
let foo : B = { bar = (25, inner) };

assert(foo.bar.0 == 25);

assert(foo.bar.1.zap == 42);

assert((0,((1,1,2), (3,5), 8), 12).1.1.1 == 5);

// Slight imbalance: between DOT and ID we can have whitespace...

assert(foo. bar .1 . zap == 42);

// but not between DOT and NUM:

// assert(foo.bar. 2.zap == 42) // Error

// N.B.: We did not change the FLOAT syntax:

let (f, g, h) : (Float, Float, Float) = (1., 1.7, 1.8e-4);

// N.B. these fail in wasm (AST-40)

let (k, l) : (Float, Float) = (0x644., 0x644.5P-1)
