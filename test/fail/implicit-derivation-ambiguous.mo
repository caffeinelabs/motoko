type Order = { #less; #greater; #equal };

// Head-level ambiguity: two derivable candidates for [T]
module Nat1 {
  public func compare(a : Nat, b : Nat) : Order {
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module Array1 {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
    #equal;
  };
};

module Array2 {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
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

ignore needsCompare([1], [2]); // ambiguous head: Array1.compare and Array2.compare
