//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  var retained : [[var Nat]] = [];

  func verify() {
    assert retained.size() == 32;
    var index = 0;
    for (value in retained.values()) {
      assert value.size() == 131072;
      assert value[0] == index;
      assert value[131071] == index;
      index += 1;
    };
  };

  public func prepare() : async () {
    // Callback allocations grow memory while the outer array is being filled.
    retained := Prim.fixtureTabulate<[var Nat]>(32, func(index) {
      Prim.Array_init<Nat>(131072, index)
    });
    verify();
  };

  public func collectAndCheck() : async () {
    var index = 0;
    while (index < 4) {
      ignore Prim.Array_init<Nat>(1024 * 1024, index);
      await async {};
      verify();
      index += 1;
    };
    Prim.debugPrint("plugin tabulation survived callbacks, memory growth and GC");
  };
};

//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
