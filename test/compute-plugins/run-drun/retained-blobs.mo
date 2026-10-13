//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/echo --plugin ../../../_build/compute-plugin-tests/typed
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  var input : Blob = "";
  var output : Blob = "";
  var shape : Blob = "";
  var reclaimedBefore = 0;

  func verify() {
    assert input.size() == 65_537;
    assert output == input;
    assert Prim.fixtureEcho(input) == output;
    assert shape == "Bn";
    assert Prim.fixtureShape(output, 1 : Nat) == shape;
  };

  public func prepare() : async () {
    input := Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(65_537,
      func(i) { Prim.natToNat8((i * 131 + 17) % 256) }));
    output := Prim.fixtureEcho(input);
    shape := Prim.fixtureShape(output, 1 : Nat);
    verify();
  };

  public func churn() : async () {
    reclaimedBefore := Prim.rts_reclaimed();
    // Move allocation to another GC partition so the retained blobs can move.
    var i = 0;
    while (i < 9) {
      ignore Prim.Array_init<Nat>(1024 * 1024, i);
      i += 1;
    };
  };

  public func collectAndCheck() : async () {
    var i = 0;
    while (i < 4) {
      await async {};
      verify();
      i += 1;
    };
    assert Prim.rts_reclaimed() > reclaimedBefore + 32 * 1024 * 1024;
    Prim.debugPrint("compute plugin fixtures survived GC");
  };
};

//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
