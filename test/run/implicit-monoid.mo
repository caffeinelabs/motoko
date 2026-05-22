// Type-class-style programming with implicit module dictionaries.
// This allows grouping related operations into a single module to be resolved implicitly.
// A Monoid bundles an identity element and an associative binary operation.
// Nat has two monoids (additive and multiplicative), demonstrating disambiguation.

type Monoid<T> = module {
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

module Nat {
  public module MonoidAdd {
    public let empty : Nat = 0;
    public func combine(a : Nat, b : Nat) : Nat { a + b };
  };

  public module MonoidMul {
    public let empty : Nat = 1;
    public func combine(a : Nat, b : Nat) : Nat { a * b };
  };
};

module Text {
  public module Monoid {
    public let empty : Text = "";
    public func combine(a : Text, b : Text) : Text { a # b };
  };
};

// Text has a single Monoid, resolved implicitly
assert fold(["hello", " ", "world"]) == "hello world";
assert fold<Text>([]) == "";

// Nat has two monoids — pass explicitly
assert fold([2, 3, 4], Nat.MonoidAdd) == 9;
assert fold([2, 3, 4], Nat.MonoidMul) == 24;
assert fold<Nat>([], Nat.MonoidAdd) == 0;
assert fold<Nat>([], Nat.MonoidMul) == 1;

// Or choose via local scope binding for implicit resolution.
// In practice, users would disambiguate by importing the right module;
// the let-binding here is equivalent.
do {
  let Monoid = Nat.MonoidAdd;
  assert fold([2, 3, 4]) == 9;
};

do {
  let Monoid = Nat.MonoidMul;
  assert fold([2, 3, 4]) == 24;
};
