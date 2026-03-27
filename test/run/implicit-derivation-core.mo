//MOC-FLAG --package core $MOTOKO_CORE --implicit-package core
import Array "mo:core/Array";
import Nat "mo:core/Nat";
import Int "mo:core/Int";
import Text "mo:core/Text";
import { type Order } "mo:core/Order";

// Derive Array.compare for [Nat] from core
func compareNatArrays(
  a : [Nat],
  b : [Nat],
  compare : (implicit : ([Nat], [Nat]) -> Order),
) : Order {
  compare(a, b);
};

assert compareNatArrays([1, 2, 3], [1, 2, 3]) == #equal;
assert compareNatArrays([1, 2], [1, 3]) == #less;
assert compareNatArrays([1, 2, 3], [1, 2]) == #greater;

// Derive Array.compare for [Int]
func compareIntArrays(
  a : [Int],
  b : [Int],
  compare : (implicit : ([Int], [Int]) -> Order),
) : Order {
  compare(a, b);
};

assert compareIntArrays([-1, 0, 1], [-1, 0, 1]) == #equal;
assert compareIntArrays([-2], [-1]) == #less;

// Derive Array.compare for [Text]
func compareTextArrays(
  a : [Text],
  b : [Text],
  compare : (implicit : ([Text], [Text]) -> Order),
) : Order {
  compare(a, b);
};

assert compareTextArrays(["a", "b"], ["a", "b"]) == #equal;

// Transitive: [[Nat]]
func compareNestedNatArrays(
  a : [[Nat]],
  b : [[Nat]],
  compare : (implicit : ([[Nat]], [[Nat]]) -> Order),
) : Order {
  compare(a, b);
};

assert compareNestedNatArrays([[1, 2], [3]], [[1, 2], [3]]) == #equal;
assert compareNestedNatArrays([[1]], [[2]]) == #less;

// Derive Array.sort for [Nat]
do {
  let sorted = Array.sort<Nat>([3, 1, 2]);
  assert sorted == [1, 2, 3];
};

// Derive Array.equal for [Nat]
func arraysEqual(
  a : [Nat],
  b : [Nat],
  equal : (implicit : ([Nat], [Nat]) -> Bool),
) : Bool {
  equal(a, b);
};

assert arraysEqual([1, 2, 3], [1, 2, 3]);
assert not arraysEqual([1, 2], [1, 3]);

// Derivation inside a module body (ObjBlockE)
do {
  module CoreOps {
    public func sortNats(arr : [Nat]) : [Nat] = Array.sort<Nat>(arr);
    public func eqNatArrays(
      a : [Nat], b : [Nat],
      equal : (implicit : ([Nat], [Nat]) -> Bool),
    ) : Bool = equal(a, b);
  };

  assert CoreOps.sortNats([3, 1, 2]) == [1, 2, 3];
  assert CoreOps.eqNatArrays([1, 2, 3], [1, 2, 3]);
  assert not CoreOps.eqNatArrays([1, 2], [1, 3]);
};

// Direct implicit still preferred over derivation
do {
  var localCalled = false;
  func compare(_a : [Nat], _b : [Nat]) : Order {
    localCalled := true;
    #equal;
  };
  assert compareNatArrays([1], [2]) == #equal;
  assert localCalled;
};

//SKIP comp
