let a : [?Nat] = [null, ?1];
var x : ?Nat = ?0;
for i in a.values() { x := i };

// This tests that the iterator does proceed past the null element
switch x {
  case null { () }
  case ?y { assert (y == 1) }
};
