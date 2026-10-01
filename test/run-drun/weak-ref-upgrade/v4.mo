import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : [var Nat64] = [var]; // should be rejected

  public func test3() : async () {
  };
};
