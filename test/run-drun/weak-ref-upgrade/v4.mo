import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : [var Nat64] = [var]; // should be rejected
  var ref2 : weak [var Nat8] = Prim.allocWeakRef(Prim.Array_init<Nat8>(1, 0));

  public func test3() : async () {
  };
};
