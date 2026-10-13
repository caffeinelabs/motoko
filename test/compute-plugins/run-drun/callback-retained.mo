//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  type Cell = { var count : Nat };
  type Payload = {
    text : Text;
    numbers : [Nat];
    cell : Cell;
    read : () -> Nat;
  };
  transient var retained : ?Payload = null;
  transient var retainedTuple : ?(Payload, () -> Nat, ?Nat) = null;
  transient var calls = 0;
  transient var reclaimedBefore = 0;

  func verify() {
    switch retained {
      case null Prim.trap("no retained callback value");
      case (?value) {
        assert value.text == "callback payload ☃!!";
        assert value.numbers.size() == 4096;
        assert value.numbers[4095] == 4095;
        assert value.cell.count == 2;
        assert value.read() == 2;
        assert calls == 2;
        assert Prim.fixtureApply<Payload, Nat>(
          func (same : Payload) : Nat { same.read() }, value
        ) == 2;
      };
    };
    switch retainedTuple {
      case null Prim.trap("no retained plugin tuple");
      case (?(value, read, absent)) {
        assert value.numbers[4095] == 4095;
        assert value.cell.count == 2;
        assert read() == 2;
        assert absent == null;
      };
    };
  };

  public func prepare() : async () {
    calls := 0;
    let cell : Cell = { var count = 0 };
    let original : Payload = {
      text = "callback payload ☃";
      numbers = Prim.Array_tabulate<Nat>(4096, func (n : Nat) : Nat { n });
      cell;
      read = func () : Nat { cell.count };
    };
    retained := ?Prim.fixtureTwice<Payload>(func (value : Payload) : Payload {
      calls += 1;
      value.cell.count += 1;
      ignore Prim.Array_tabulate<Nat>(65536, func (n : Nat) : Nat { n + calls });
      let same = Prim.fixtureApply<Payload, Payload>(
        func (nested : Payload) : Payload {
          Prim.fixtureIdentity<Payload>(nested)
        }, value
      );
      { same with text = same.text # "!" }
    }, original);
    switch retained {
      case null Prim.trap("no tuple payload");
      case (?value) {
        retainedTuple := ?Prim.fixtureTriple<Payload, () -> Nat, ?Nat>(
          value, value.read, null
        );
      };
    };
    // Both versions share the original payload's mutable cell and array.
    assert original.cell.count == 2;
    assert original.text == "callback payload ☃";
    verify();
  };

  public func churn() : async () {
    reclaimedBefore := Prim.rts_reclaimed();
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
    Prim.debugPrint("plugin callback payloads survived GC and reentry");
  };
};

//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
//CALL ingress prepare "DIDL\x00\x00"
//CALL ingress churn "DIDL\x00\x00"
//CALL ingress collectAndCheck "DIDL\x00\x00"
