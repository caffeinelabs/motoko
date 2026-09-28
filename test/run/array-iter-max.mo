import Prim "mo:⛔";
// test we can iterate over vals and keys of largest array
let max_size = 2**29; // maximum array size
let a = Prim.Array_tabulate<Nat>(max_size, func i = i + 1);
var c = 0;
for i in a.values() {
  assert i == c + 1;
  c += 1;
};
assert c == max_size;
Prim.debugPrint(debug_show c);
var d = 0;
for k in a.keys() {
  assert k == d;
  d += 1;
};
assert d == max_size;
Prim.debugPrint(debug_show d);

// Prim.Array_tabulate<Nat>(max_size+1,func i = i+1); // should trap, but mangled stacktrace is noisy for testing

//SKIP run
//SKIP run-ir
//SKIP run-low
