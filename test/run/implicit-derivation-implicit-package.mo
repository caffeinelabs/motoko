//MOC-FLAG --package core $MOTOKO_CORE --implicit-package core
import { type Order } "mo:core/Order";

// Direct candidates from --implicit-package should take precedence over derived ones
// Note: this avoids breaking changes

module MyNat {
  public func compare(_ : Nat, _ : Nat, foo : (implicit : Nat)) : Order {
    ignore foo;
    #equal; // always returns #equal, unlike the real Nat.compare
  };
};

let foo : Nat = 42;
ignore foo;

func compareNats(
  a : Nat,
  b : Nat,
  compare : (implicit : (Nat, Nat) -> Order),
) : Order {
  compare(a, b);
};

// This should use the original Nat.compare, not MyNat.compare
assert compareNats(1, 2) == #less;
assert compareNats(5, 5) == #equal;
assert compareNats(3, 1) == #greater;

//SKIP comp
