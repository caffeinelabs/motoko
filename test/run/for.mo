import Prim "mo:⛔";

class range(x : Nat, y : Nat) {
  var i = x;
  public func next() : ?Nat { if i > y { null } else { let j = i; i += 1; ?j } };
};

var i = 0;
for j in range(0, 10) {
  Prim.debugPrintNat(j);
  assert(j == i);
  i += 1;
};
assert(i == 11);
