//MOC-FLAG -W M0237

// Soundness regression for M0237. Before the fix, M0237 only checked that
// implicit lookup for the parameter name resolved at the call site -- it did
// not re-run inference assuming the argument was gone. In contexts where the
// type parameter was pinned only by the contravariant removable argument
// (e.g. `(K, K) -> Order`), applying the suggestion produced M0098 on the
// very form M0237 had just suggested.
//
// The fix re-runs inference in pre mode with the explicit-implicit args
// replaced by holes; M0237 fires only if the resulting instantiation matches.

type Order = { #less; #equal; #greater };

module Nat {
  public func compare(_ : Nat, _ : Nat) : Order { #equal };
};

module Iter {
  public type Iter<T> = { next : () -> ?T };
  public func fromArray<T>(_ : [T]) : Iter<T> { { next = func() = null } };
};

module Map {
  // `var` keeps K invariant here -> K cannot flow from the result type
  // through a fresh wrapper type variable.
  public type Map<K, V> = { var kv : ?(K, V) };
  public func fromIter<K, V>(
    iter : Iter.Iter<(K, V)>,
    compare : (implicit : (K, K) -> Order),
  ) : Map<K, V> {
    ignore iter; ignore compare; { var kv = null }
  };
};

func id<A>(a : A) : A { a };

// Sound: removal still typechecks in a void context -> M0237 fires.
ignore Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare);

// Unsound before the fix: removal would underconstrain K under the `id<A>`
// wrapper. M0237 must NOT fire here.
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare));

// The (genuinely ill-typed) form the old M0237 used to suggest. Kept to pin
// the M0098 the fix is meant to *prevent the compiler from recommending*.
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([])));
