//MOC-FLAG -W M0237

// Regression test for the M0237 soundness bug (https://github.com/dfinity/motoko/issues/TODO).
//
// M0237 ("argument can be inferred and omitted") only checks that implicit
// lookup for the parameter name resolves at the call site -- it does not
// re-run inference assuming the argument is gone.
//
// When the type parameter `K` is pinned ONLY by the contravariant
// `(K, K) -> Order` argument (i.e. it is in an invariant position in the
// remaining args / return type), removing the explicit argument leaves `K`
// underconstrained. moc then emits M0098 on the very form M0237 just told the
// user to write.
//
// EXPECTED today (buggy): the line below the warning produces M0098.
// EXPECTED after fix    : M0237 should NOT fire on the wrapped call.

type Order = { #less; #equal; #greater };

module Nat {
  public func compare(_ : Nat, _ : Nat) : Order { #equal };
};

module Iter {
  public type Iter<T> = { next : () -> ?T };
  public func fromArray<T>(_ : [T]) : Iter<T> { { next = func() = null } };
};

module Map {
  // `var` makes K invariant in this position -> inference can't recover it
  // from the result type alone.
  public type Map<K, V> = { var kv : ?(K, V) };
  public func fromIter<K, V>(
    iter : Iter.Iter<(K, V)>,
    compare : (implicit : (K, K) -> Order),
  ) : Map<K, V> {
    ignore iter; ignore compare; { var kv = null }
  };
};

func id<A>(a : A) : A { a };

// 1) Standalone -- M0237 fires and the suggestion is sound (removal type-checks).
ignore Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare);

// 2) Wrapped in a polymorphic context -- M0237 still fires...
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([]), Nat.compare));

// 3) ...but applying the suggestion produces M0098. This is the bug.
ignore id(Map.fromIter(Iter.fromArray<(Nat, Text)>([])));
