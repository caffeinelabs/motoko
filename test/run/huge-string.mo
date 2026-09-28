import Prim "mo:⛔";

/*
This uses repeated self-concatenation to build a huge string
that would fill up most of the memory if implemented naively.
It then checks that heap usage is actually not too bad,
showing that string concatenation works as a tree with sharing.
*/

class range(x : Nat, y : Nat) {
  var i = x;
  public func next() : ?Nat { if i > y { null } else { let j = i; i += 1; ?j } };
};

let before = Prim.rts_heap_size();
var s = "Badger";
let n = 27;
for i in range(1, n) {
  s := s # s;
};
let after = Prim.rts_heap_size();
assert(+after - before < 2_000);

//SKIP run
//SKIP run-low
//SKIP run-ir
