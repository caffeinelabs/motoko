import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak[var Nat64] = Prim.allocWeakRef(blob);

  public func test3() : async () {
    assert (not Prim.isLive(ref));
  };
};
