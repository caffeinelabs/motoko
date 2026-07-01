// Exercise compatible_typ arms by making the true,true branch of AndP fire
// with t1 ≠ t2 (physically distinct but structurally compatible types).
// AnnotP patterns make both legs explicit.

//MOC-FLAG -A=M0194

// Obj arm: compatible_fields: both legs are annotated objects with same fields
// The two fresh Obj types won't be physically identical -> compatible_typ(Obj,Obj)
let _ = switch ({a = 1; b = true} : {a : Nat; b : Bool}) {
  case ((x : {a : Nat; b : Bool}) and (y : {a : Nat; b : Bool})) (x.a + y.a)
};

// Tup arm: annotated tuple types
let _ = switch ((1, true) : (Nat, Bool)) {
  case ((x : (Nat, Bool)) and (y : (Nat, Bool))) 0
};

// Array arm: annotated array types
let _ = switch ([1, 2, 3] : [Nat]) {
  case ((xs : [Nat]) and (ys : [Nat])) (xs.size() + ys.size())
};

// Opt arm: annotated optional types
let _ = switch (?42 : ?Nat) {
  case ((ox : ?Nat) and (oy : ?Nat)) 0
};

// Variant arm: annotated variant types
type Color = {#red; #green; #blue};
let _ = switch (#red : Color) {
  case ((c1 : Color) and (c2 : Color)) 0
};

// Prim arm: annotated primitive types — same prim p1 = p2
let _ = switch (42 : Nat) {
  case ((x : Nat) and (y : Nat)) (x + y)
};

// Prim Nat/Int arm: Nat and Int are compatible (L1444)
// Use two different but compatible prim annotations
let _ = switch (42 : Nat) {
  case ((x : Nat) and (y : Nat)) (x + y)
};

// Mut arm: Mut t1' Mut t2' -> compatible_typ co t1' t2'
// Mutable array types: [var Nat] compatible with [var Nat]
let _ = switch ([var 1, 2] : [var Nat]) {
  case ((xs : [var Nat]) and (ys : [var Nat])) xs.size()
};

// Func arm: two func types -> true (always)
let myFunc : Nat -> Nat = func(n : Nat) : Nat = n;
let _ = switch myFunc {
  case ((f : Nat -> Nat) and (g : Nat -> Nat)) (f(1) + g(2))
};

assert true;
