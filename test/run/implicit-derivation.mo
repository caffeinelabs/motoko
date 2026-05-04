type Order = { #less; #greater; #equal };

// Base compare functions (monomorphic, act as leaf implicits)
var natCompareCalls = 0;
var intCompareCalls = 0;
var arrayCompareCalls = 0;

module Nat {
  public func compare(a : Nat, b : Nat) : Order {
    natCompareCalls += 1;
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module Int {
  public func compare(a : Int, b : Int) : Order {
    intCompareCalls += 1;
    if (a < b) #less else if (a == b) #equal else #greater;
  };
};

module Text {
  public func compare(_a : Text, _b : Text) : Order {
    #equal;
  };
};

// Polymorphic higher-order compare (has implicit parameter)
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

// Basic derivation with [Nat]
func compareNatArrays(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
  compare(a, b);
};

natCompareCalls := 0;
arrayCompareCalls := 0;
assert compareNatArrays([1, 2, 3], [1, 2, 3]) == #equal;
assert arrayCompareCalls == 1;
assert natCompareCalls == 3;

assert compareNatArrays([1, 2], [1, 3]) == #less;

// Derivation with [Int]
func compareIntArrays(a : [Int], b : [Int], compare : (implicit : ([Int], [Int]) -> Order)) : Order {
  compare(a, b);
};

intCompareCalls := 0;
arrayCompareCalls := 0;
assert compareIntArrays([1, 2, 3], [1, 2, 3]) == #equal;
assert arrayCompareCalls == 1;
assert intCompareCalls == 3;

// Explicit still works alongside derivation
func myArrayCompare(a : [Nat], b : [Nat]) : Order {
  Array.compare<Nat>(a, b, Nat.compare);
};
assert compareNatArrays([1, 2], [1, 3], myArrayCompare) == #less;

// Derivation with [Text]
func compareTextArrays(a : [Text], b : [Text], compare : (implicit : ([Text], [Text]) -> Order)) : Order {
  compare(a, b);
};

assert compareTextArrays(["a"], ["b"]) == #equal;

// Direct implicit still preferred over derivation
do {
  var localCalled = false;
  func compare(_a : [Nat], _b : [Nat]) : Order {
    localCalled := true;
    #equal;
  };

  assert compareNatArrays([1, 2], [1, 3]) == #equal;
  assert localCalled;
};

// Monomorphic derivation (no type params)
module Pair {
  public func compare(a : (Nat, Nat), b : (Nat, Nat), compare : (implicit : (Nat, Nat) -> Order)) : Order {
    let c1 = compare(a.0, b.0);
    switch (c1) {
      case (#equal) { compare(a.1, b.1) };
      case _ c1;
    };
  };
};

func comparePairs(a : (Nat, Nat), b : (Nat, Nat), compare : (implicit : ((Nat, Nat), (Nat, Nat)) -> Order)) : Order {
  compare(a, b);
};

assert comparePairs((1, 2), (1, 3)) == #less;
assert comparePairs((1, 2), (1, 2)) == #equal;

// Polymorphic function uses derived implicit at call site
func polySort<T>(a : [T], b : [T], compare : (implicit : ([T], [T]) -> Order)) : Order {
  compare(a, b);
};

assert polySort<Nat>([1, 2], [1, 3]) == #less;

// Multiple implicits on the same function (one derived, one direct)
module Hasher {
  public func hash<T>(x : T, hash : (implicit : T -> Nat)) : Nat {
    hash(x);
  };
};

module Nat2 {
  public func hash(x : Nat) : Nat {
    x;
  };
};

func needsBothImplicits(
  a : [Nat],
  b : [Nat],
  compare : (implicit : ([Nat], [Nat]) -> Order),
  hash : (implicit : Nat -> Nat),
) : (Order, Nat) { (compare(a, b), hash(42)) };

let (ord, h) = needsBothImplicits([1], [2]);
assert ord == #less;
assert h == 42;

// Local module shadowing
// Local module's compare should win over outer Array.compare derivation
do {
  var localArrayCalled = false;
  module Array {
    public func compare(_a : [Nat], _b : [Nat]) : Order {
      localArrayCalled := true;
      #equal;
    };
  };

  func needsCompare(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare([1], [2]) == #equal;
  assert localArrayCalled;
};

// Direct resolution with zero non-implicit candidate in scope
module Const {
  public func compare<T>(compare : (implicit : (T, T) -> Order)) : (T, T) -> Order {
    compare;
  };
};

func needsConstCompare(compare : (implicit : (Nat, Nat) -> Order)) : (Nat, Nat) -> Order {
  compare;
};

let cmp = needsConstCompare();
assert cmp(1, 2) == #less;

// Multiple type parameters with multiple inner implicits
do {
  module PairCmp {
    public func compare<A, B>(
      a : (A, B),
      b : (A, B),
      cmpA : (implicit : (compare : (A, A) -> Order)),
      cmpB : (implicit : (compare : (B, B) -> Order)),
    ) : Order {
      let c1 = cmpA(a.0, b.0);
      switch (c1) {
        case (#equal) { cmpB(a.1, b.1) };
        case _ c1;
      };
    };
  };

  func compareMixedPairs(
    a : (Nat, Int),
    b : (Nat, Int),
    compare : (implicit : ((Nat, Int), (Nat, Int)) -> Order),
  ) : Order {
    compare(a, b);
  };

  assert compareMixedPairs((1, -2), (1, -3)) == #greater;
  assert compareMixedPairs((1, -2), (1, -2)) == #equal;
  assert compareMixedPairs((1, -2), (2, -2)) == #less;
};

// Derivation from local scope (top-level func, not module field)
do {
  var localDeriveCalled = false;
  func compare<T>(_ : [T], _ : [T], compare : (implicit : (T, T) -> Order)) : Order {
    ignore compare;
    localDeriveCalled := true;
    #equal;
  };

  func needsCompare(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare([1], [2]) == #equal;
  assert localDeriveCalled;
};

// Local-derived (from local val) preferred over derived from module field
do {
  var localDeriveCalled = false;
  var moduleDeriveCalled = false;

  module _ArrMod {
    public func compare<T>(_ : [T], _ : [T], compare : (implicit : (T, T) -> Order)) : Order {
      ignore compare;
      moduleDeriveCalled := true;
      #equal;
    };
  };

  func compare<T>(_ : [T], _ : [T], compare : (implicit : (T, T) -> Order)) : Order {
    ignore compare;
    localDeriveCalled := true;
    #equal;
  };

  func needsCompare(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare([1], [2]) == #equal;
  assert localDeriveCalled;
  assert not moduleDeriveCalled;
};

// Direct local val preferred over direct module field
do {
  var localDirectCalled = false;
  var moduleDirectCalled = false;

  module _M {
    public func compare(_ : Nat, _ : Nat) : Order {
      moduleDirectCalled := true;
      #equal;
    };
  };

  func compare(_ : Nat, _ : Nat) : Order {
    localDirectCalled := true;
    #equal;
  };

  func needsCompare(a : Nat, b : Nat, compare : (implicit : (Nat, Nat) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare(1, 2) == #equal;
  assert localDirectCalled;
  assert not moduleDirectCalled;
};

// Direct module field preferred over derived local
do {
  var directModuleCalled = false;
  var derivedLocalCalled = false;

  module M {
    public func compare(_ : [Nat], _ : [Nat]) : Order {
      directModuleCalled := true;
      #equal;
    };
  };

  func compare<T>(_ : [T], _ : [T], compareT : (implicit : (compare : (T, T) -> Order))) : Order {
    ignore compareT;
    derivedLocalCalled := true;
    #equal;
  };
  ignore compare;

  func needsCompare(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare([1], [2]) == #equal;
  assert directModuleCalled;
  assert not derivedLocalCalled;
};

// Direct module field preferred over derived module field
do {
  var directModuleCalled = false;
  var derivedModuleCalled = false;

  module DirectM {
    public func compare(_ : [Nat], _ : [Nat]) : Order {
      directModuleCalled := true;
      #equal;
    };
  };

  module _DerivedM {
    public func compare<T>(_ : [T], _ : [T], compare : (implicit : (T, T) -> Order)) : Order {
      ignore compare;
      derivedModuleCalled := true;
      #equal;
    };
  };

  func needsCompare(a : [Nat], b : [Nat], compare : (implicit : ([Nat], [Nat]) -> Order)) : Order {
    compare(a, b);
  };

  assert needsCompare([1], [2]) == #equal;
  assert directModuleCalled;
  assert not derivedModuleCalled;
};

// Subtyping in derivation: inner implicit resolved via supertype (Int.compare for Nat args)
do {
  var intCompareCalled = false;

  // Shadow outer modules so only local definitions are in scope
  module Nat {};
  module Int {
    public func compare(a : Int, b : Int) : Order {
      intCompareCalled := true;
      if (a < b) #less else if (a == b) #equal else #greater;
    };
  };

  module Array {
    public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order {
      let len = a.size();
      if (len != b.size()) {
        if (len < b.size()) #less else #greater;
      } else {
        var i = 0;
        var result : Order = #equal;
        label l while (i < len) {
          switch (compare(a[i], b[i])) {
            case (#equal) {};
            case other { result := other; break l };
          };
          i += 1;
        };
        result;
      };
    };
  };

  func needsNatArrayCompare(
    a : [Nat],
    b : [Nat],
    compare : (implicit : ([Nat], [Nat]) -> Order),
  ) : Order { compare(a, b) };

  // Int.compare : (Int, Int) -> Order satisfies (Nat, Nat) -> Order via Nat <: Int
  assert needsNatArrayCompare([1, 2], [1, 3]) == #less;
  assert intCompareCalled;
};
