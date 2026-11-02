type Order = { #less; #greater; #equal };

var natCompareCalls = 0;
var arrayCompareCalls = 0;

module Nat {
  public func compare(a : Nat, b : Nat) : Order {
    natCompareCalls += 1;
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module Array {
  public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
    arrayCompareCalls += 1;
    let len = a.size();
    if (len != b.size()) {
      if (len < b.size()) #less else #greater;
    } else {
      var i = 0;
      var result : Order = #equal;
      label l while (i < len) {
        let c = compare(a[i], b[i]);
        switch (c) {
          case (#equal) {};
          case _ { result := c; break l };
        };
        i += 1;
      };
      result;
    };
  };
};

// Transitive derivation: [[Nat]] -> [Nat] -> Nat
// Array.compare<[Nat]> needs [Nat].compare, which is Array.compare<Nat>,
// which needs Nat.compare
func compareNestedArrays(
  a : [[Nat]],
  b : [[Nat]],
  compare : (implicit : ([[Nat]], [[Nat]]) -> Order),
) : Order {
  compare(a, b);
};

natCompareCalls := 0;
arrayCompareCalls := 0;
assert compareNestedArrays([[1, 2], [3]], [[1, 2], [3]]) == #equal;
assert arrayCompareCalls == 3; // 1 outer [[Nat]] + 2 inner [Nat] comparisons
assert natCompareCalls == 3; // (1,1) (2,2) (3,3)

natCompareCalls := 0;
arrayCompareCalls := 0;
assert compareNestedArrays([[1]], [[2]]) == #less;
assert arrayCompareCalls == 2; // outer [[Nat]] + inner [Nat]
assert natCompareCalls == 1;

natCompareCalls := 0;
arrayCompareCalls := 0;
assert compareNestedArrays([[1, 2], [3]], [[1, 2], [4]]) == #less;
assert arrayCompareCalls == 3; // outer + 2 inner
assert natCompareCalls == 3; // 1==1, 2==2, 3<4

// Derivation inside a module body (ObjBlockE)
do {
  module NestedOps {
    public func compareNested(
      a : [[Nat]],
      b : [[Nat]],
      compare : (implicit : ([[Nat]], [[Nat]]) -> Order),
    ) : Order = compare(a, b);
  };

  natCompareCalls := 0;
  arrayCompareCalls := 0;
  assert NestedOps.compareNested([[1, 2], [3]], [[1, 2], [3]]) == #equal;
  assert arrayCompareCalls == 3;
  assert natCompareCalls == 3;

  assert NestedOps.compareNested([[1]], [[2]]) == #less;
};
