//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  type A = Prim.PreparedA;
  type B = Prim.PreparedB;
  type Held = { key : A; read : () -> Blob };
  transient var retained : ?Held = null;
  transient var keys : [A] = [];
  transient var large : ?A = null;
  transient var other : ?B = null;
  transient var expected : Blob = "";
  transient var reclaimedBefore = 0;
  transient var heapWithKeys = 0;

  func verify() {
    assert keys.size() == 16;
    for (key in keys.values()) assert Prim.preparedLenA(key) == 131_073;
    switch large {
      case null Prim.trap("missing large prepared payload");
      case (?key) assert Prim.preparedLenA(key) == 64 * 1024 * 1024 + 1;
    };
    assert Prim.preparedReadA(keys[0]) ==
      Prim.arrayMutToBlob(Prim.Array_init<Nat8>(131_073, 0));
    assert Prim.preparedReadA(keys[15]) ==
      Prim.arrayMutToBlob(Prim.Array_init<Nat8>(131_073, 15));
    switch retained {
      case null Prim.trap("missing prepared key");
      case (?held) {
        assert Prim.preparedLenA(held.key) == 65_537;
        assert held.read() == expected;
        let result = Prim.fixtureApply<A, A>(func(key : A) : A {
          ignore Prim.Array_init<Nat>(16_384, 19);
          assert Prim.preparedReadA(key) == expected;
          Prim.fixtureIdentity<A>(key)
        }, held.key);
        assert Prim.preparedReadA(result) == expected;
      };
    };
    switch other {
      case null Prim.trap("missing second prepared type");
      case (?key) assert Prim.preparedReadB(key) == "\00\ff\80\01\00";
    };
  };

  public func prepare() : async () {
    expected := Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(65_537,
      func(i) { Prim.natToNat8((i * 131 + 17) % 256) }));
    let key = Prim.preparedA(expected);
    retained := ?{ key; read = func() : Blob { Prim.preparedReadA(key) } };
    keys := Prim.Array_tabulate<A>(16, func(i) {
      Prim.preparedA(Prim.arrayMutToBlob(
        Prim.Array_init<Nat8>(131_073, Prim.natToNat8(i))))
    });
    // A >64 MiB leaf uses large-object partitions. Unlike small garbage in
    // the active allocation partition, it can be reclaimed without waiting
    // for further allocation pressure after the final reference is dropped.
    var text = ".";
    var size = 0;
    while (size < 26) {
      text #= text;
      size += 1;
    };
    large := ?Prim.preparedA(Prim.encodeUtf8(text # "!"));
    other := ?Prim.preparedB("\00\ff\80\01\00");
    verify();
  };

  public func churn() : async () {
    reclaimedBefore := Prim.rts_reclaimed();
    // Move allocations into another partition so retained payloads can move.
    var i = 0;
    while (i < 9) {
      ignore Prim.Array_init<Nat>(1024 * 1024, i);
      i += 1;
    };
  };

  public func collectAndCheck() : async () {
    let acrossAwait = switch retained {
      case null Prim.trap("missing async prepared key");
      case (?held) held.key;
    };
    var i = 0;
    while (i < 6) {
      await async {};
      assert Prim.preparedReadA(acrossAwait) == expected;
      verify();
      i += 1;
    };
    assert Prim.rts_reclaimed() > reclaimedBefore + 32 * 1024 * 1024;
    // Settle temporary read copies before recording the retained heap.
    await async {};
    await async {};
    heapWithKeys := Prim.rts_heap_size();
    assert heapWithKeys >= 2 * 1024 * 1024;
    Prim.debugPrint("prepared aliases survived GC and nested callbacks");
  };

  public func release() : async () {
    reclaimedBefore := Prim.rts_reclaimed();
    retained := null;
    keys := [];
    large := null;
    other := null;
    expected := "";
  };

  public func checkReleased() : async () {
    var i = 0;
    while (i < 8) {
      await async {};
      i += 1;
    };
    let reclaimed = Prim.rts_reclaimed();
    let heapAfter = Prim.rts_heap_size();
    assert reclaimed > reclaimedBefore + 64 * 1024 * 1024;
    // Messages and GC also allocate; allow 1 MiB for their bookkeeping.
    assert heapAfter + 63 * 1024 * 1024 < heapWithKeys;
    Prim.debugPrint("unreferenced prepared payloads were reclaimed");
  };
};

//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
//CALL ingress release "DIDL\x00\x00"
//CALL ingress checkReleased "DIDL\x00\x00"
//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
//CALL ingress release "DIDL\x00\x00"
//CALL ingress checkReleased "DIDL\x00\x00"
