//MOC-FLAG -W M0237

// M0237 must not fire when removing the implicit would underconstrain K.

type Order = { #less; #equal; #greater };

module Nat {
  public func compare(_ : Nat, _ : Nat) : Order { #equal };
};

module Iter {
  public type Iter<T> = { next : () -> ?T };
  public func fromArray<T>(_ : [T]) : Iter<T> { { next = func() = null } };
};

module Map {
  public type Map<K, V> = { var kv : ?(K, V) };
  public func fromIter<K, V>(
    iter : Iter.Iter<(K, V)>,
    compare : (implicit : (K, K) -> Order),
  ) : Map<K, V> {
    ignore iter; ignore compare; { var kv = null }
  };
};

func id<A>(a : A) : A { a };

// Void context: removal typechecks -> M0237 fires.
ignore Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare);

// Wrapped: removal would underconstrain K -> M0237 must NOT fire.
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare));

// Pins the M0098 the old M0237 used to suggest.
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([])));
