//MOC-FLAG -A=M0145
import { debugPrint } = "mo:⛔";

// simplest: both legs succeed, two disjoint bindings on the same value
let (a : Nat) and (b : Nat) = 5;
debugPrint (debug_show {a; b});

// option pattern + bare name — ?x constrains, s captures the whole value
let (?x) and s = ?7;
debugPrint (debug_show {x; s});

// nested and: three legs bind three names to the same value
let (x3 : Nat) and (y3 : Nat) and (z3 : Nat) = 11;
debugPrint (debug_show {x3; y3; z3});

// and mixed with or — `and` binds tighter than `or`, so this groups as
// `((#a n) and m) or ((#b n) and m) : ...`
let ((#a n) and m) or ((#b n) and m) : { #a : Nat; #b : Nat } = #a 13;
debugPrint (debug_show {n; m});

//SKIP run-ir
//SKIP run-low
//SKIP comp
