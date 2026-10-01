import Prim "mo:prim";

actor {

  var blob = Prim.Array_init<Nat64>(3, 1);
  var ref : weak[var Nat64] = Prim.allocWeakRef(blob);

  public func test2() : async () {
    // Should be live.
    assert (Prim.isLive(ref));

    // Overwrite the blob to check if it's collected by the GC.
    blob := Prim.Array_init<Nat64>(3, 2);

    var n = 10;
    while (n > 0) {
      n -= 1;
      // Trigger a GC by allocating memory and then by yielding.
      let new_array = Prim.Array_init<Nat64>(1024 * 1024 * 10, 3);
      await async {};
    };

    // Should be dead after GC cleaning up the blob.
    assert (not Prim.isLive(ref));
  };
};
