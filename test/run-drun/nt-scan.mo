import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// entries, keys and values hand back every row exactly once, in batches: tables of 0 rows, of 1
// row, and of 5,000 rows across more than one row group with 1,000 of them removed. keys and
// values give the keys and rows of entries, in the same order, and reverseEntries those of entries
// in the reverse order. Rows come in the order their keys were first added
actor {
  type Row = { name : Text; square : Nat };

  let none = NativeTable.empty<Nat, Row>().build();
  let one = NativeTable.empty<Text, Nat>().build();
  let many = NativeTable.empty<Nat, Row>().build();

  func row(k : Nat) : Row = { name = "row " # debug_show k; square = k * k };

  // 1,000 of the 5,000 keys are removed
  func removed(k : Nat) : Bool = k % 5 == 2;

  public func put() : async () {
    one.add("only", 1);
    var k = 0;
    while k < 5_000 {
      many.add(k, row(k));
      k += 1
    };
    k := 0;
    while k < 5_000 {
      if removed(k) { many.remove(k) };
      k += 1
    };
    Prim.debugPrint(debug_show (none.size(), one.size(), many.size()))
  };

  public query func scan() : async () {
    Prim.debugPrint(debug_show (none.entries().next(), none.keys().next(), none.values().next()));

    let (e, k, v) = (one.entries(), one.keys(), one.values());
    Prim.debugPrint(debug_show (e.next(), k.next(), v.next()));
    Prim.debugPrint(debug_show (e.next(), k.next(), v.next()));

    let seen = Prim.Array_init<Bool>(5_000, false);
    let (keys, values) = (many.keys(), many.values());
    var rows = 0;
    var wrong = 0;
    var twice = 0;
    var disagree = 0;
    for (k, r) in many.entries() {
      rows += 1;
      if k >= 5_000 or removed(k) or r != row(k) {
        wrong += 1
      } else {
        if seen[k] { twice += 1 };
        seen[k] := true
      };
      if keys.next() != ?k { disagree += 1 };
      if values.next() != ?r { disagree += 1 }
    };
    var missing = 0;
    var i = 0;
    while i < 5_000 {
      if not removed(i) and not seen[i] { missing += 1 };
      i += 1
    };
    Prim.debugPrint(debug_show { rows; wrong; twice; missing; disagree });
    Prim.debugPrint(debug_show (keys.next(), values.next()));

    Prim.debugPrint(debug_show (none.reverseEntries().next(), one.reverseEntries().next()));
    let added = Prim.Array_init<Nat>(rows, 0);
    var n = 0;
    for k in many.keys() { added[n] := k; n += 1 };
    for (k, r) in many.reverseEntries() {
      n -= 1;
      if k != added[n] or r != row(k) { disagree += 1 }
    };
    Prim.debugPrint(debug_show { n; disagree })
  };

  // add over a key keeps its row's place, a key removed and added again comes last; a scan newest
  // first hands out no row added or removed since it began, and every other row once
  public func order() : async () {
    many.add(3, row(3));
    many.remove(4);
    many.add(4, row(4));
    let names : NativeTable.NativeTable<Nat, { name : Text }> = many.view();
    let (old, back) = (many.keys(), names.reverseEntries());
    Prim.debugPrint(debug_show (old.next(), old.next(), old.next(), back.next(), back.next()));
    many.add(6_000, row(6_000));
    many.remove(0);
    var rows = 2;
    var wrong = 0;
    for (k, _) in back { rows += 1; if k == 6_000 or k == 0 { wrong += 1 } };
    Prim.debugPrint(debug_show (rows, wrong, many.size()))
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress scan "DIDL\x00\x00"
//CALL ingress order "DIDL\x00\x00"
