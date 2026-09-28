import Prim "mo:prim";

actor {
  var number : Nat = 12345678901234567890123456789012345678901234567890123456789012345678901234567890;

  public func modify() : async () {
    number += 1;
  };

  public func print() : async () {
    Prim.debugPrint(debug_show (number));
  };
};
