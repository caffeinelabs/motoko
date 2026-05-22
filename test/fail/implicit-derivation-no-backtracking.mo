// Two candidates (ArrayA.compare and ArrayB.compare) both structurally match
// ([Nat], [Nat]) -> Order after erasing implicits.
// With commit-first resolution, this is an ambiguity error regardless of
// whether inner implicits can be resolved.

type Order = { #less; #greater; #equal };

module Nat {
  public func compare(a : Nat, b : Nat) : Order {
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module ArrayA {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
    #equal;
  };
};

// ArrayB also matches structurally, but its inner implicit has a different name.
module ArrayB {
  public func compare<T>(a : [T], b : [T], cmp : (implicit : (T, T) -> Order)) : Order {
    #less;
  };
};

func needsCompare(
  a : [Nat],
  b : [Nat],
  compare : (implicit : ([Nat], [Nat]) -> Order),
) : Order {
  compare(a, b);
};

// ambiguous: ArrayA.compare and ArrayB.compare both match as initial candidates
// even though ArrayB needs `cmp` implicit that is NOT available in scope and would fail to resolve.
// with backtracking we could resolve and pick ArrayA, but it would make the resolution fragile
// because importing/defining `cmp` would suddenly break the resolution.
ignore needsCompare([1], [2]);
