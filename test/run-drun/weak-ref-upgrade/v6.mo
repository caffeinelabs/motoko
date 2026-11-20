import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak [var Nat64] = Prim.allocWeakRef(blob);
  var ref2 : weak [var Nat16] = Prim.allocWeakRef(Prim.Array_init<Nat16>(1, 0)); // should be rejected

  public func test3() : async () {}
}
