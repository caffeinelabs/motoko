import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak[var Nat64] = Prim.allocWeakRef(blob);

  public func test1() : async () {
    // Should be live.
    assert (Prim.isLive(ref));
  };

};
