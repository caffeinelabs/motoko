//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  type Record = {
    value : Text; count : Nat64; children : [?Text];
    pair : (Nat8, Bool); nested : ??Text;
  };
  var retained : ?Record = null;
  var reclaimedBefore = 0;

  func verify() {
    switch retained {
      case null Prim.trap("no retained value");
      case (?value) {
        assert value.value == "retained text ☃";
        assert value.count == 17;
        assert value.children.size() == 257;
        assert value.children[0] == null;
        assert value.children[256] == ?value.value;
        assert value.pair == (255, true);
        assert value.nested == ?null;
      };
    };
  };

  public func prepare() : async () {
    switch (Prim.fixtureRecord("retained text ☃")) {
      case (#ok value) retained := ?value;
      case (#err message) Prim.trap(message);
    };
    verify();
  };

  public func churn() : async () {
    reclaimedBefore := Prim.rts_reclaimed();
    var i = 0;
    while (i < 9) {
      ignore Prim.Array_init<Nat>(1024 * 1024, i);
      // Exercise a recoverable return while a result array is being filled.
      assert Prim.fixtureEarlyError("temporary") == #err("early error");
      i += 1;
    };
  };

  public func collectAndCheck() : async () {
    var i = 0;
    while (i < 4) {
      await async {};
      verify();
      assert Prim.fixtureEarlyError("temporary") == #err("early error");
      i += 1;
    };
    assert Prim.rts_reclaimed() > reclaimedBefore + 32 * 1024 * 1024;
    Prim.debugPrint("plugin managed results survived GC and early errors");
  };
};

//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
