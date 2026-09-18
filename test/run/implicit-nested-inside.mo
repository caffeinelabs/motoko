// Inside `module Hash`, its nested modules are in scope by themselves, so the
// enclosing module must not offer the same bindings a second time as
// `Hash.Nat.hash` and turn the natural in-module use into an ambiguity.

module Hash {
  public module Nat { public func hash(x : Nat) : Nat = x + 1 };
  public module Text { public func hash(x : Text) : Nat = x.size() };

  public func apply<T>(x : T, hash : (implicit : T -> Nat)) : Nat = hash(x);

  public func n() : Nat = apply(1);
  public func t() : Nat = apply("abc");

  public module Deeper {
    // Both Hash and Deeper enclose this call
    public func n() : Nat = apply(2);
  };
};

assert Hash.n() == 2;
assert Hash.t() == 3;
assert Hash.Deeper.n() == 3;

// Outside the module the nested search applies as usual
assert Hash.apply(5) == 6;
assert Hash.apply("hello") == 5;
