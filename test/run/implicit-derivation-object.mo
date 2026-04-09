//MOC-FLAG --package core $MOTOKO_CORE
// Implicit derivation of non-function (object) values via classes/functions.
// Tests typeclass-style dictionaries bundled as objects.

import Array "mo:core/Array";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import { type Order } "mo:core/Order";

// --- Ord typeclass as an object type ---

type Ord<T> = {
  equal : (T, T) -> Bool;
  compare : (T, T) -> Order;
};

module NatEx {
  public let Ord : Ord<Nat> = object {
    public func equal(a : Nat, b : Nat) : Bool { Nat.equal(a, b) };
    public func compare(a : Nat, b : Nat) : Order { Nat.compare(a, b) };
  };
};

module TextEx {
  public let Ord : Ord<Text> = object {
    public func equal(a : Text, b : Text) : Bool { Text.equal(a, b) };
    public func compare(a : Text, b : Text) : Order { Text.compare(a, b) };
  };
};

// Derived Ord for arrays — delegates to core Array utilities
module ArrayEx {
  public func Ord<T>(Ord : (implicit : Ord<T>)) : Ord<[T]> = object {
    public func equal(a : [T], b : [T]) : Bool { Array.equal(a, b, Ord.equal) };
    public func compare(a : [T], b : [T]) : Order { Array.compare(a, b, Ord.compare) };
  };
};

func isEqual<T>(a : T, b : T, Ord : (implicit : Ord<T>)) : Bool { Ord.equal(a, b) };
func cmp<T>(a : T, b : T, Ord : (implicit : Ord<T>)) : Order { Ord.compare(a, b) };

// Direct resolution: Ord<Nat> → NatEx.Ord
assert isEqual(1, 1);
assert not isEqual(1, 2);
assert cmp(1, 2) == #less;
assert cmp(3, 2) == #greater;

// Direct resolution: Ord<Text> → TextEx.Ord
assert isEqual("a", "a");
assert not isEqual("a", "b");

// Derived: Ord<[Nat]> → ArrayEx.Ord(NatEx.Ord)
assert isEqual([1, 2, 3], [1, 2, 3]);
assert not isEqual([1, 2], [1, 3]);
assert cmp([1, 2], [1, 3]) == #less;
assert cmp([1, 2, 3], [1, 2]) == #greater;

// Derived: Ord<[Text]> → ArrayEx.Ord(TextEx.Ord)
assert isEqual(["a", "b"], ["a", "b"]);
assert not isEqual(["a"], ["b"]);

// Transitive: Ord<[[Nat]]> → ArrayEx.Ord(ArrayEx.Ord(NatEx.Ord))
assert isEqual([[1, 2], [3]], [[1, 2], [3]]);
assert not isEqual([[1, 2], [3]], [[1, 2], [4]]);
assert cmp([[1]], [[2]]) == #less;

// Explicit still works
assert isEqual<Nat>(1, 1, NatEx.Ord);
assert cmp<[Nat]>([1], [2], ArrayEx.Ord<Nat>(NatEx.Ord)) == #less;

// --- Monoid typeclass ---

type Monoid<T> = {
  empty : T;
  combine : (T, T) -> T;
};

func fold<T>(xs : [T], Monoid : (implicit : Monoid<T>)) : T {
  var acc = Monoid.empty;
  for (x in xs.vals()) {
    acc := Monoid.combine(acc, x);
  };
  acc;
};

// Array concatenation monoid — no element Monoid needed (zero-implicit derivation)
module ArrayEx2 {
  public func Monoid<T>() : Monoid<[T]> = object {
    public let empty : [T] = [];
    public func combine(a : [T], b : [T]) : [T] { Array.concat(a, b) };
  };
};

// Derived: Monoid<[Nat]> → ArrayEx2.Monoid<Nat>()
assert fold<[Nat]>([[], [1, 2], [3]]).size() == 3;
assert fold<[Nat]>([[1], [2, 3]]) == [1, 2, 3];
assert fold<[Nat]>([]) == [];
