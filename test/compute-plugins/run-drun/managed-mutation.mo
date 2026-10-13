//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/mutation
//MOC-FLAG --sanity-checks --force-gc
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

actor {
  type Payload = { data : [var Nat]; text : Text };
  type Cell = { var value : Payload; var spare : Payload; marker : Nat };
  let initial : Payload = { data = [var 7]; text = "initial" };
  let cells : [var Cell] = Prim.fixtureMutTabulate<Cell>(32,
    func(_) { Prim.fixtureMutRecord<Payload>(initial) });
  let aliases : [var Payload] = Prim.fixtureMutTabulate<Payload>(32,
    func(_) { initial });
  var live : [var [var Nat]] = [var];
  var generation = 0;
  var sawPendingCycle = false;
  var sawCompletedCycle = false;
  let options = Prim.fixtureMutTabulate<??Null>(1, func(_) { null });

  func verify() {
    assert initial.data[0] == 7;
    for (i in cells.keys()) {
      let payload = Prim.fixtureMutRead<Payload>(cells[i]);
      assert payload.data[0] == generation * 100 + i;
      assert payload.data[payload.data.size() - 1] == generation * 100 + i;
      assert aliases[i].data[0] == generation * 100 + i;
      assert cells[i].spare.data[0] == 7;
      assert Prim.fixtureMutMarker(cells[i]) == 7;
    };
    assert options[0] == ??null;
  };

  public func build() : async () {
    // Like gc-cache-upgrade, >96 MiB of live objects exceeds the incremental
    // GC's work budget, keeping collection active across ingress messages.
    live := Prim.fixtureMutTabulate<[var Nat]>(192, func(i) {
      // Dead blocks between live ones also exercise evacuation/forwarding.
      ignore Prim.Array_init<Nat>(64 * 1024, 0);
      Prim.Array_init<Nat>(64 * 1024, i)
    });
    assert Prim.rts_heap_size() > 192 * 1024 * 1024;
    assert live[191][65535] == 191;
  };

  public func mutate() : async () {
    // max_live is updated only when a cycle completes. With forced GC at
    // every message boundary, this proves writes occur before it completes.
    sawPendingCycle := sawPendingCycle or
      (Prim.rts_heap_size() > 96 * 1024 * 1024 and
       Prim.rts_max_live_size() < 96 * 1024 * 1024);
    sawCompletedCycle := sawCompletedCycle or
      Prim.rts_max_live_size() >= 96 * 1024 * 1024;
    generation += 1;
    for (i in cells.keys()) {
      let payload : Payload = {
        data = Prim.Array_init<Nat>(4096, generation * 100 + i);
        text = "new payload";
      };
      Prim.fixtureMutWrite<Payload>(cells[i], payload);
      Prim.fixtureMutSet<Payload>(aliases, i, payload);
    };
    Prim.fixtureMutSet<??Null>(options, 0, ??null);
    verify();
    assert live[0][65535] == 0 and live[191][65535] == 191;
    assert Prim.fixtureMutEarlyArray() == "early";
    assert Prim.fixtureMutEarlyRecord() == "early";
  };

  public func finish() : async () {
    var count = 0;
    while (count < 12) {
      ignore Prim.Array_init<Nat>(131072, count);
      await async {};
      verify();
      count += 1;
    };
    assert sawPendingCycle;
    assert sawCompletedCycle or Prim.rts_max_live_size() >= 96 * 1024 * 1024;
    Prim.debugPrint("managed mutation retained aliases through incremental GC");
  };
};

//CALL ingress build "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress mutate "DIDL\x00\x00"
//CALL ingress finish "DIDL\x00\x00"
