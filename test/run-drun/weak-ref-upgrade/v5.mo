import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak Any = Prim.allocWeakRef<Any>({}); // should be rejected

  public func test3() : async () {
  };
};
