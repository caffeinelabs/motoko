import Prim "mo:prim";

// Same shape as v1, so the upgrade is memory-compatible and the heap carries over.

persistent actor {

  var live : [var [var Nat]] = Prim.Array_init<[var Nat]>(192, Prim.Array_init<Nat>(0, 0));

  public func build() : async () {
    for (i in live.keys()) {
      live[i] := Prim.Array_init<Nat>(64 * 1024, i);
    };
  };

  public func churn() : async () {
    for (i in live.keys()) {
      live[i] := Prim.Array_init<Nat>(16 * 1024, i);
    };
    assert (live.size() == 192);
  };

  public func size() : async Nat = async live.size();
}
