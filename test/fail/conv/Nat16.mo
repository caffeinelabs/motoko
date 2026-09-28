import Prim "mo:⛔"
module {

  public func toNat(n : Nat16) : Nat { Prim.nat16ToNat(n) };

  public func fromNat16(n : Nat16) : Nat { Prim.nat16ToNat(n) };

  public module Outer {
    public module Inner {
      public func toNat(n : Nat16) : Nat { Prim.nat16ToNat(n) };
      public func fromNat16(n : Nat16) : Nat { Prim.nat16ToNat(n) };
    }
  }

}
