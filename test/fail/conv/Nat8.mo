import Prim "mo:⛔"
module {

  public func toNat(n : Nat8) : Nat { Prim.nat8ToNat(n) };

  public func fromNat8(n : Nat8) : Nat { Prim.nat8ToNat(n) };

  public module Outer {
    public module Inner {
      public func toNat(n : Nat8) : Nat { Prim.nat8ToNat(n) };
      public func fromNat8(n : Nat8) : Nat { Prim.nat8ToNat(n) };
    }
  }

}
