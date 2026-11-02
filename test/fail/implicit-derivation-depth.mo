//MOC-FLAG --implicit-derivation-depth 1

type Order = { #less; #greater; #equal };

module Nat {
  public func compare(a : Nat, b : Nat) : Order {
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module Array {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
    #equal;
  };
};

// [[Nat]] requires two levels of derivation:
// 1. Array.compare<[Nat]> (depth 0 -> 1)
// 2. Array.compare<Nat>   (depth 1 -> 2, but blocked by limit=1)
func needsNestedArrayCompare(
  a : [[Nat]],
  b : [[Nat]],
  compare : (implicit : ([[Nat]], [[Nat]]) -> Order),
) : Order {
  compare(a, b);
};

ignore needsNestedArrayCompare([[1]], [[2]]);
