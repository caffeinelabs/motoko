//MOC-FLAG -A=M0194 -W=M0236
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

// Ambiguous case but caused by the difference between inferring vs checking
module Tricky1 {
  public func tricky(self : Int) : Text { "int" # debug_show self };
};
module Tricky2 {
  public func tricky(self : Nat) : Text { "nat" # debug_show self };
};
func id<T>(a : T) : T { a };
assert id(1).tricky() == "nat1"; // Inferred as `Nat` because `id(1)` is in the inferring position so it defaults to `Nat`
assert Tricky1.tricky(id(1)) == "int+1"; // Inferred as `Int` because `1` is in the checking position of `Int`
// We should NOT suggest rewriting here because it would change the resolution!
