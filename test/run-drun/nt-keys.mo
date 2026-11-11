import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// Keys of several fields: a table whose key is a tuple of 2 or 3 scalars writes, reads and removes
// rows by the whole key, in rows of the open group and in packed rows, and a key that differs from
// a row's in one part finds no row. Scans, index walks and views give the key as the tuple, a part
// too long to lie in one piece of table memory included; update and upsert set fields by the key;
// evolve keeps the key. The cells of a scan hold no key that is a tuple
actor {
  type Item = { name : Text; qty : Nat };
  type Noted = { name : Text; qty : Nat; note : ?Text };

  let items = NativeTable.empty<(Text, Nat), Item>().createIndex(["qty"]).build();
  let names : NativeTable.NativeTable<(Text, Nat), { name : Text }> = items.view();
  let moves = NativeTable.empty<(Char, Int, Text), Int>().build();

  func key(i : Nat) : (Text, Nat) = ("doc-" # debug_show (i % 10), i / 10);
  func item(i : Nat) : Item = { name = "item " # debug_show i; qty = i % 7 };

  // A Text of 3,000 bytes: past what a row holds in one piece
  func long(c : Text) : Text {
    var t = "";
    var i = 0;
    while i < 3_000 { t #= c; i += 1 };
    t
  };

  public func put() : async () {
    items.add((long("p"), 0), item(0));
    var i = 0;
    while i < 4_200 { items.add(key(i), item(i)); i += 1 };
    items.add((long("o"), 1), item(1));
    Prim.debugPrint(debug_show (moves.insert(('x', -5, "a"), 10), moves.insert(('x', -5, "b"), 11), moves.insert(('y', -5, "a"), 12)));
    Prim.debugPrint(debug_show (moves.insert(('x', 5, "a"), 13), moves.insert(('x', -5, "a"), 14), moves.size()))
  };

  public query func get() : async () {
    Prim.debugPrint(debug_show (items.get(key(5)), items.get(key(4_199)), items.get(("doc-5", 1)), items.get(("doc-6", 0)), items.get(("doc-5", 4_200))));
    Prim.debugPrint(debug_show (moves.get(('x', -5, "a")), moves.get(('x', -5, "b")), moves.get(('y', -5, "a")), moves.get(('x', 5, "a")), moves.get(('x', -5, "c")), moves.get(('z', -5, "a"))));
    var found = 0;
    var i = 0;
    while i < 4_200 {
      if items.get(key(i)) == ?item(i) and names.get(key(i)) == ?({ name = item(i).name }) and items.containsKey(key(i)) {
        found += 1
      };
      i += 1
    };
    Prim.debugPrint(debug_show (found, items.size(), items.get((long("p"), 0)), items.get((long("o"), 1)) == ?item(1), names.get((long("p"), 0))))
  };

  // Every entry's row is the row of its key, the keys and views give the same keys, and the
  // long keys come back whole
  public query func scans() : async () {
    var rows = 0;
    var same = 0;
    var longs = 0;
    for (k, r) in items.entries() {
      rows += 1;
      if items.get(k) == ?r { same += 1 };
      if k.0.size() == 3_000 { longs += 1 }
    };
    var keys = 0;
    let all = items.keys();
    for (k, n) in names.entries() {
      if all.next() == ?k and (do ? { items.get(k)!.name }) == ?n.name { keys += 1 }
    };
    Prim.debugPrint(debug_show (rows, same, longs, keys, all.next()));
    let entries = moves.entries();
    Prim.debugPrint(debug_show (entries.next(), entries.next(), entries.next(), entries.next(), entries.next()))
  };

  // The index walks give each row with its key, through the table and through a view
  public query func walks() : async () {
    let three = ?[#nat(3)];
    var rows = 0;
    for (k, r) in items.indexEntries(["qty"], three, three) {
      if r.qty == 3 and items.get(k) == ?r { rows += 1 }
    };
    let viewed = names.indexEntries(["qty"], three, three);
    var same = 0;
    for (k, _) in items.indexEntries(["qty"], three, three) {
      switch viewed.next() {
        case ?(k2, n) { if k == k2 and ?n.name == (do ? { items.get(k)!.name }) { same += 1 } }
        case null {}
      }
    };
    Prim.debugPrint(debug_show (rows, same, items.count(["qty"], [#nat(3)]), items.minEntry(["qty"]) == ?((long("p"), 0), item(0)), names.maxEntry(["qty"])))
  };

  public func writes() : async () {
    Prim.debugPrint(debug_show (items.update(key(5), { qty = 6 }), items.update(("doc-5", 4_200), { qty = 6 }), items.get(key(5))));
    Prim.debugPrint(debug_show (items.delete(key(7)), items.delete(key(7)), items.get(key(7)), items.get(key(17))));
    items.remove((long("p"), 0));
    items.remove(key(4_190));
    Prim.debugPrint(debug_show (items.get((long("p"), 0)), items.get(key(4_190)), items.size(), items.count(["qty"], [#nat(6)])));
    Prim.debugPrint(debug_show (items.cells(false, ["qty", "name"], null).next(), items.fieldTypes()))
  };

  public func change() : async () {
    let t : NativeTable.NativeTable<(Text, Nat), Noted> = items.evolve();
    t.add(("new", 0), { name = "new"; qty = 1; note = ?"added" });
    Prim.debugPrint(debug_show (t.get(key(6)), t.get(("new", 0)), t.get((long("o"), 1)) == ?{ item(1) with note = null : ?Text }, t.size()));
    let rows = t.entries();
    Prim.debugPrint(debug_show (rows.next(), t.fieldTypes()));
    Prim.debugPrint(debug_show (t.upsert(("up", 0), { name = "up"; qty = 2 }), t.upsert(("up", 0), { name = "up"; qty = 3 }), t.get(("up", 0))))
  };

  public func cellsWithKey() : async () {
    ignore moves.cells(true, [], null).next()
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress scans "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress writes "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress cellsWithKey "DIDL\x00\x00"
