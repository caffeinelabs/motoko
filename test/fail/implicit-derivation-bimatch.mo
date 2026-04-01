// Tests for bi-matching limits in implicit candidate selection.

// B is fully phantom (no bounds at all); C has an upper bound from ret_subs
// (C ≤ Nat) but no lower bound — bivariant solver still picks lb = None for both.
module Pipeline {
  public func chain<A, B, C>(
    x : A,
    step : (implicit : A -> B),
    finish : (implicit : B -> C),
  ) : C { finish(step(x)) };
};

func needsChain(x : Nat, chain : (implicit : Nat -> Nat)) : Nat { chain(x) };
ignore needsChain(42);

// Over-constrained: arg_subs gives T ≥ Nat, ret_subs gives T ≤ Bool.
// Nat </: Bool → impossible → candidate silently dropped; no derivation note in the error.
module Wrapper {
  public func wrap<T>(x : T, fn : (implicit : T -> T)) : T { fn(x) };
};

func needsWrap(x : Nat, wrap : (implicit : Nat -> Bool)) : Bool { wrap(x) };
ignore needsWrap(42);
