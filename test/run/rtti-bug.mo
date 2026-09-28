//MOC-FLAG --sanity-checks

import { debugPrint } "mo:prim";

class range(x : Nat, y : Int) {
  var i = x;
  public func next() : ?Nat {
    if i > y { null } else { let j = i; i += 1; ?j }
  }
};

for _ in range(0, 11) { debugPrint "." };

//SKIP run
//SKIP run-ir
//SKIP run-low
