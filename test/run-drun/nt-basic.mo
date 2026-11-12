import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// Rows written with add and insert read back with get: they equal the rows written, print the
// same, and their fields and patterns work as on any record. insert says whether the key is new,
// and the sizes count each key once. Rows removed with remove and delete are gone, the other rows
// stay, and a removed key is new again
//
// Keys of several fields: a table whose key is a tuple of 2 or 3 scalars writes, reads and removes
// rows by the whole key, in rows of the open group and in packed rows, and a key that differs from
// a row's in one part finds no row. Scans, index walks and views give the key as the tuple, a part
// too long to lie in one piece of table memory included; update and upsert set fields by the key;
// evolve keeps the key. The cells of a scan hold no key that is a tuple
actor {
  type User = { name : Text; city : Text; amount : Nat };
  type Item = { name : Text; qty : Nat };
  type Noted = { name : Text; qty : Nat; note : ?Text };

  let users = NativeTable.empty<Nat, User>().build();
  let places = NativeTable.empty<Text, Text>().build();
  let counts = NativeTable.empty<Text, Nat>().build();

  let items = NativeTable.empty<(Text, Nat), Item>().createIndex(["qty"]).build();
  let names : NativeTable.NativeTable<(Text, Nat), { name : Text }> = items.view();
  let moves = NativeTable.empty<(Char, Int, Text), Int>().build();

  func key(i : Nat) : (Text, Nat) = ("doc-" # debug_show (i % 10), i / 10);
  func item(i : Nat) : Item = { name = "item " # debug_show i; qty = i % 7 };

  // A Text of 131,072 bytes with two-byte characters, built by concatenation
  func long() : Text {
    var t = "Zürich ";
    var i = 0;
    while i < 14 { t := t # t; i += 1 };
    t
  };

  // A Text of 3,000 bytes: past what a row holds in one piece
  func longPart(c : Text) : Text {
    var t = "";
    var i = 0;
    while i < 3_000 { t #= c; i += 1 };
    t
  };

  public func put() : async () {
    users.add(7, { name = "Ana"; city = "Basel"; amount = 40 });
    Prim.debugPrint(debug_show users.insert(8, { name = "Bo"; city = "Bern"; amount = 0 }));
    // Numbers on both sides of the compact form and of 64 bits
    users.add(2_305_843_009_213_693_951, { name = "Cy"; city = "Chur"; amount = 2_305_843_009_213_693_951 });
    users.add(2 ** 61, { name = "Cy"; city = "Chur"; amount = 2 ** 61 });
    users.add(2 ** 63, { name = "Di"; city = "Davos"; amount = 2 ** 63 });
    users.add(18_446_744_073_709_551_615, { name = "Di"; city = "Davos"; amount = 18_446_744_073_709_551_615 });
    users.add(2 ** 100, { name = "Ed"; city = "Ems"; amount = 2 ** 100 });
    // Long values: in one piece of table memory, and spread over more than one
    users.add(10, { name = "Fay"; city = "Faido"; amount = 2 ** 20_000 });
    users.add(11, { name = "Gil"; city = long(); amount = 2 ** 460_000 });
    // A record with more fields than the row type
    users.add(12, { name = "Hal"; city = "Horw"; amount = 5; active = true; var visits = 1 });
    // A replaced row
    Prim.debugPrint(debug_show users.insert(8, { name = "Bo"; city = "Biel"; amount = 1 }));

    places.add("Zürich, " # "the largest city of Switzerland", "on the " # "Limmat");
    places.add("Genève", "on the Rhône");
    places.add("", "");
    Prim.debugPrint(debug_show places.insert("", ""));
    counts.add("Basel", 3);
    counts.add("Bern", 2 ** 70);
    Prim.debugPrint(debug_show (users.size(), places.size(), counts.size()));
    // Keys of several fields
    items.add((longPart("p"), 0), item(0));
    var i = 0;
    while i < 4_200 { items.add(key(i), item(i)); i += 1 };
    items.add((longPart("o"), 1), item(1));
    Prim.debugPrint(debug_show (moves.insert(('x', -5, "a"), 10), moves.insert(('x', -5, "b"), 11), moves.insert(('y', -5, "a"), 12)));
    Prim.debugPrint(debug_show (moves.insert(('x', 5, "a"), 13), moves.insert(('x', -5, "a"), 14), moves.size()))
  };

  public query func get() : async () {
    // ==, debug_show, field access and pattern matching
    let ana = users.get(7);
    Prim.debugPrint(debug_show ana);
    Prim.debugPrint(debug_show (ana == ?{ name = "Ana"; city = "Basel"; amount = 40 }));
    switch ana {
      case ?{ name; city = "Basel"; amount } { Prim.debugPrint(name # " " # debug_show (amount + 2)) }
      case _ { Prim.debugPrint("no match") }
    };
    switch users.get(8) {
      case ?bo { Prim.debugPrint(bo.name # " lives in " # bo.city # ", amount " # debug_show bo.amount) }
      case null { Prim.debugPrint("missing") }
    };
    for n in [2_305_843_009_213_693_951, 2 ** 61, 2 ** 63, 18_446_744_073_709_551_615, 2 ** 100].values() {
      switch users.get(n) {
        case ?u { Prim.debugPrint(debug_show (u.amount == n, u.amount * 2 == n + n, u)) }
        case null { Prim.debugPrint("missing " # debug_show n) }
      }
    };
    let fay = users.get(10);
    Prim.debugPrint(debug_show (fay == ?{ name = "Fay"; city = "Faido"; amount = 2 ** 20_000 }));
    switch users.get(11) {
      case ?gil { Prim.debugPrint(debug_show (gil.city == long(), gil.city.size(), gil.amount == 2 ** 460_000)) }
      case null { Prim.debugPrint("missing") }
    };
    Prim.debugPrint(debug_show users.get(12));
    Prim.debugPrint(debug_show (users.get(9), users.get(2 ** 62), users.get(2 ** 101)));
    Prim.debugPrint(debug_show (users.containsKey(7), users.containsKey(9), users.containsKey(2 ** 100)));

    // Scalar rows of Text and Nat
    Prim.debugPrint(debug_show places.get("Zürich, the largest city of Switzerland"));
    Prim.debugPrint(debug_show (places.get("Genève") == ?"on the Rhône", places.get(""), places.get("Bern")));
    Prim.debugPrint(debug_show (counts.get("Basel"), counts.get("Bern"), counts.get("Chur")));
    switch counts.get("Bern") {
      case ?n { Prim.debugPrint(debug_show (n / 2 ** 69)) }
      case null { Prim.debugPrint("missing") }
    };
    Prim.debugPrint(debug_show (places.containsKey(""), counts.containsKey("Chur")));
    // Keys of several fields
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
    Prim.debugPrint(debug_show (found, items.size(), items.get((longPart("p"), 0)), items.get((longPart("o"), 1)) == ?item(1), names.get((longPart("p"), 0))))
  };

  public func remove() : async () {
    // delete says whether the key had a row
    Prim.debugPrint(debug_show (users.delete(7), users.delete(7), users.delete(9)));
    users.remove(8);
    users.remove(8);
    users.remove(9);
    // Keys on both sides of the compact form and of 64 bits, rows with long values
    users.remove(2 ** 61);
    Prim.debugPrint(debug_show (users.delete(2 ** 63), users.delete(2 ** 101), users.delete(2 ** 100)));
    Prim.debugPrint(debug_show (users.delete(11), users.delete(10), users.delete(11)));
    Prim.debugPrint(debug_show (users.size(), users.containsKey(7), users.containsKey(11), users.containsKey(12)));
    // Text keys, one built by concatenation
    Prim.debugPrint(debug_show (places.delete("Zürich, the " # "largest city of Switzerland"), places.delete("Zürich"), places.delete("")));
    counts.remove("Basel");
    counts.remove("Bern");
    Prim.debugPrint(debug_show (places.size(), counts.size(), counts.isEmpty(), places.isEmpty()));
    Prim.debugPrint(debug_show (places.get("Genève"), places.get(""), counts.get("Bern")));
    // A removed key is new again, and reads back its new row
    Prim.debugPrint(debug_show (users.insert(7, { name = "Ana"; city = "Aarau"; amount = 41 }), counts.insert("Bern", 4), places.insert("", "-")));
    Prim.debugPrint(debug_show (users.get(7), counts.get("Bern"), places.get("")));
    Prim.debugPrint(debug_show (users.size(), places.size(), counts.size()))
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
    Prim.debugPrint(debug_show (rows, same, items.count(["qty"], [#nat(3)]), items.minEntry(["qty"]) == ?((longPart("p"), 0), item(0)), names.maxEntry(["qty"])))
  };

  public func writes() : async () {
    Prim.debugPrint(debug_show (items.update(key(5), { qty = 6 }), items.update(("doc-5", 4_200), { qty = 6 }), items.get(key(5))));
    Prim.debugPrint(debug_show (items.delete(key(7)), items.delete(key(7)), items.get(key(7)), items.get(key(17))));
    items.remove((longPart("p"), 0));
    items.remove(key(4_190));
    Prim.debugPrint(debug_show (items.get((longPart("p"), 0)), items.get(key(4_190)), items.size(), items.count(["qty"], [#nat(6)])));
    Prim.debugPrint(debug_show (items.cells(false, ["qty", "name"], null).next(), items.fieldTypes()))
  };

  public func change() : async () {
    let t : NativeTable.NativeTable<(Text, Nat), Noted> = items.evolve();
    t.add(("new", 0), { name = "new"; qty = 1; note = ?"added" });
    Prim.debugPrint(debug_show (t.get(key(6)), t.get(("new", 0)), t.get((longPart("o"), 1)) == ?{ item(1) with note = null : ?Text }, t.size()));
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
//CALL ingress remove "DIDL\x00\x00"
//CALL ingress writes "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress cellsWithKey "DIDL\x00\x00"
