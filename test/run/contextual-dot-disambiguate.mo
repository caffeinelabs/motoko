//MOC-FLAG -A=M0194
module Int {
  public func eq(self : Int, other : Int) : Bool { assert false; self == other };
};

module Nat {
  public func eq(self : Nat, other : Nat) : Bool { self == other };
};

let s : Nat = 41;
assert s.eq(s); // Disambiguates to Nat, because Nat <: Int

type A = {};
type B = { a : Int };
type C = { a : Int; b : Text };

let a : A = {};
let b : B = { a = 0 };
let c : C = { a = 0; b = "" };

module MA {
  public func cmp(self : A, _ : A) : Nat = 1;
};

module MB {
  public func cmp(self : B, _ : B) : Nat = 2;
};

module MC {
  public func cmp(self : C, _ : C) : Nat = 3;
};

assert a.cmp(b) == 1;
assert b.cmp(c) == 2;
assert c.cmp(c) == 3;
