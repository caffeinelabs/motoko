import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// 200,000 rows of about 1 KiB are added in batches, so the table takes store partitions beyond
// the one it starts in, between rounds of heap garbage: arrays of which every eighth stays alive
// until the next round, and one array larger than a partition. The program runs with the debug
// runtime's checks, and the collector runs after every message. Every row then reads back right,
// in batches between more rounds of garbage, and the arrays kept alive stay intact
actor {
  type Row = { text : Text; n : Nat };

  let rows = NativeTable.empty<Nat, Row>().build();

  let batch = 20_000;
  var added = 0;
  var checked = 0;

  // 1,000 bytes every row's text ends with
  let pad : Text = do {
    var t = "";
    var i = 0;
    while i < 100 {
      t #= "0123456789";
      i += 1
    };
    t
  };

  func row(k : Nat) : Row = { text = debug_show k # pad; n = 3 * k + 1 };

  // the arrays the last round of garbage kept alive, and that round's number
  let kept = Prim.Array_init<[Nat]>(512, []);
  var round = 0;

  func element(round : Nat, slot : Nat, i : Nat) : Nat = round * 1_000_000 + slot * 1_000 + i;

  // the store partitions the table takes while rows are added, each of which adds 64 MiB to the
  // heap size at once
  var partitions = 0;

  public func add() : async () {
    let stop = added + batch;
    while added < stop {
      let r = row(added);
      let before = Prim.rts_heap_size();
      rows.add(added, r);
      if Prim.rts_heap_size() >= before + 64 * 1024 * 1024 { partitions += 1 };
      added += 1
    };
    Prim.debugPrint(debug_show { added; size = rows.size(); partitions })
  };

  public func garbage() : async () {
    var wrong = 0;
    var slot = 0;
    while slot < kept.size() {
      let array = kept[slot];
      if round > 0 {
        if array.size() != 1_000 { wrong += 1 } else {
          var i = 0;
          while i < 1_000 {
            if array[i] != element(round, slot, i) { wrong += 1 };
            i += 1
          }
        }
      };
      slot += 1
    };
    round += 1;
    var a = 0;
    while a < 8 * kept.size() {
      let array = Prim.Array_tabulate<Nat>(1_000, func i = element(round, a / 8, i));
      if a % 8 == 0 { kept[a / 8] := array };
      a += 1
    };
    // 72,000,000 bytes, more than a partition of 64 MiB
    let large = Prim.Array_init<Nat>(9_000_000, round);
    Prim.debugPrint(debug_show { round; wrong; large = large.size() })
  };

  public func check() : async () {
    let stop = checked + batch;
    var wrong = 0;
    while checked < stop {
      if rows.get(checked) != ?row(checked) { wrong += 1 };
      checked += 1
    };
    Prim.debugPrint(debug_show { checked; wrong; size = rows.size() })
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --sanity-checks
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress add "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress garbage "DIDL\x00\x00"
