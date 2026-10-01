type A = {};
type B = { a : Int };
type C = { a : Int; b : Text };

let a : A = {};
let b : B = { a = 0 };
let c : C = { a = 0; b = "" };

module Contravariant {
  module MA {
    public func cmp(_ : A, _ : A) : Nat = 1;
  };

  module MB {
    public func cmp(_ : B, _ : B) : Nat = 2;
  };

  module MC {
    public func cmp(_ : C, _ : C) : Nat = 3;
  };

  func cmp<T>(x : T, y : T, cmp : (implicit : (T, T) -> Nat)) : Nat {
    cmp(x, y);
  };

  public func test() {
    assert cmp(a, b) == 1;
    assert cmp(b, c) == 2;
    assert cmp(c, c) == 3;
  };
};

module Covariant {
  module MA {
    public let default : A = {};
  };

  module MB {
    public let default : B = { a = 0 };
  };

  module MC {
    public let default : C = { a = 0; b = "" };
  };

  func default<T>(default : (implicit : T)) : T = default;

  public func test() {
    let _ : A = default<A>();
    assert default<B>().a == 0;
    assert default<C>().b == "";
  };
};

Contravariant.test();
Covariant.test();
