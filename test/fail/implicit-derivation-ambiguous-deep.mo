type Order = { #less; #greater; #equal };

// Single derivable candidate for [T] — no head-level ambiguity
module Array {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
    #equal;
  };
};

// Two leaf implicits for Nat — inner resolution is ambiguous
module Nat {
  public func compare(a : Nat, b : Nat) : Order {
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module MyNat {
  public func compare(a : Nat, b : Nat) : Order {
    #greater;
  };
};

func needsCompare(
  a : [Nat],
  b : [Nat],
  compare : (implicit : ([Nat], [Nat]) -> Order),
) : Order {
  compare(a, b);
};

ignore needsCompare([1], [2]); // Array.compare is unique head, but inner Nat compare is ambiguous
