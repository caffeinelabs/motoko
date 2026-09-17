import Prim "mo:⛔";
// test we can iterate over vals and keys of largest mutable array
let max_size = 2**29; // maximum array size
let a = Prim.Array_init<Nat>(max_size, 666);
var c1 = 0;
for (v in a.values()) {
 assert v == 666; c1 += 1;
};
var c2 = 0;
for (v in a.values()) {
 assert v == 666; c2 += 1;
};
assert c1 == c2;
Prim.debugPrint(debug_show c1);
var d = 0;
for (k in a.keys()) {
  assert k == d; d += 1;
};
assert d == max_size;
Prim.debugPrint(debug_show d);

// Prim.Array_init<Nat>(max_size+1,0); // should trap, but mangled stacktrace is noisy for testing

//SKIP run
//SKIP run-ir
//SKIP run-low
