type Order = { #less; #greater; #equal };

// Only Nat.compare is available as a leaf implicit
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

// Missing inner implicit
// Derivation of [Bool] compare needs Bool.compare, which doesn't exist
func needsBoolArrayCompare(
  a : [Bool],
  b : [Bool],
  compare : (implicit : ([Bool], [Bool]) -> Order),
) : Order {
  compare(a, b);
};

ignore needsBoolArrayCompare([true], [false]); // M0230: no Bool.compare

// Deep nesting: [[Bool]] needs two derivation levels, both shown in diagnostics
func needsNestedBoolArrayCompare(
  a : [[Bool]],
  b : [[Bool]],
  compare : (implicit : ([[Bool]], [[Bool]]) -> Order),
) : Order {
  compare(a, b);
};

ignore needsNestedBoolArrayCompare([[true]], [[false]]); // M0230: full chain shown

// Non-function implicit cannot be derived
func needsNatValue(x : Nat, n : (implicit : Nat)) : Nat {
  n;
};

ignore needsNatValue(42); // M0230: no Nat value in scope
