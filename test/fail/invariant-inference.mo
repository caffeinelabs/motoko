import Prim "mo:⛔";
// illustrate inconsistency of non-principal mutable array literal inference and
// principal type parameter inference
do {
  let a = Prim.Array_init(1, 0); // rejected as ambiguous
};

do {
  let a = Prim.Array_init(1, 0) : [var Int]; // accepted
  a[0] := -1;
};

do {
  let a = [var 0]; // accepted as [var Nat], but did the programmer actually intend [var Int]?
  a[0] := -1; // rejected
};

do {
  let a = [var 0] : [var Int]; // accepted as [var Int]
  a[0] := -1; // accepted
};

do {
  let a : [var Int] = [var 0]; // ditto, accepted as [var Int]
  a[0] := -1; // accepted
};
