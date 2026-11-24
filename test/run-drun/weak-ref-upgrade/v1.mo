import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak[var Nat64] = Prim.allocWeakRef(blob);
  var ref2 : weak [var Nat8] = Prim.allocWeakRef(Prim.Array_init<Nat8>(1, 0));

  public func test1() : async () {
    // Should be live.
    assert (Prim.isLive(ref));
  };

};
