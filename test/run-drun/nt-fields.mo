import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// A view reads some fields of each row: its get gives the record of the view's fields, equal to
// those fields of the row get gives, for rows still in the open group, packed rows and replaced
// packed rows, long values, a table whose row type changed, a view of a view, a view of no field
// and a view of a scalar row. Its scans and index walks give the same rows as the table's, with
// the view's fields; its counts, sums, extremes, filters and cells name any field of the table;
// delete on a view removes the whole row. A view of a field the row lacks or has with another
// type, add and evolve on a view, and a view made before an evolve trap, and change nothing.
// update sets some fields of a row and keeps the others, in packed rows and rows of the open
// group, with long values, and in a table whose row type changed; the index on a changed field
// moves and the totals change by the difference; a Nat is taken for an Int. upsert inserts a row,
// null in the fields it leaves out, when the key has none, and is update when the key has one. An
// update of a field the row lacks or has with another type, and an update on a view, trap and
// change nothing
actor {
  type Order = { customer : Text; city : Text; amount : Nat; paid : Bool; placed : Int; note : ?Text };
  type Name = { customer : Text; city : Text };
  type Money = { amount : Nat; note : ?Text };

  let orders = NativeTable.empty<Nat, Order>().createIndex(["city"]).build();
  let names : NativeTable.NativeTable<Nat, Name> = orders.view();
  let money : NativeTable.NativeTable<Nat, Money> = orders.view();
  let cities : NativeTable.NativeTable<Nat, { city : Text }> = names.view();
  let keys : NativeTable.NativeTable<Nat, {}> = orders.view();
  let all : NativeTable.NativeTable<Nat, Order> = orders.view();

  let scores = NativeTable.empty<Nat, { score : Int; best : ?Int; tag : Text }>().createIndex(["score"]).build();

  let counts = NativeTable.empty<Text, Nat>().build();
  let sameCounts : NativeTable.NativeTable<Text, Nat> = counts.view();

  type Old = { name : Text; age : Nat; town : Text };
  type New = { name : Text; age : Nat; email : ?Text };
  let people = NativeTable.empty<Nat, Old>().build();
  let oldNames : NativeTable.NativeTable<Nat, { name : Text }> = people.view();
  var changed : ?NativeTable.NativeTable<Nat, New> = null;

  // A Text of 131,072 bytes with two-byte characters, built by concatenation
  func long() : Text {
    var t = "Zürich ";
    var i = 0;
    while i < 14 { t := t # t; i += 1 };
    t
  };

  func order(i : Nat) : Order {
    {
      customer = "c" # debug_show i;
      city = if i % 3 == 0 { "Basel" } else { "Bern" };
      amount = i * 7;
      paid = i % 2 == 0;
      placed = -i;
      note = if i % 5 == 0 { null } else { ?("n" # debug_show i) }
    }
  };

  // 4,200 rows: rows 0 to 4,095 fill a group, which is packed; the rest stay in the open group
  public func put() : async () {
    var i = 0;
    while i < 4_200 { orders.add(i, order(i)); i += 1 };
    // A replaced packed row, long values in a packed row and in the open group
    orders.add(7, { order(7) with city = "Chur"; note = ?"replaced" });
    orders.add(8, { order(8) with customer = long(); amount = 2 ** 20_000 });
    orders.add(4_150, { order(4_150) with customer = long(); amount = 2 ** 460_000 });
    counts.add("Basel", 3);
    counts.add("Bern", 2 ** 70);
    scores.add(1, { score = -3; best = ?(-1); tag = "a" });
    people.add(1, { name = "Ana"; age = 40; town = "Basel" });
    people.add(2, { name = "Bo"; age = 30; town = "Bern" })
  };

  public query func get() : async () {
    Prim.debugPrint(debug_show (names.get(0), names.get(7), names.get(4_199), names.get(99_999)));
    Prim.debugPrint(debug_show (money.get(5), money.get(7), cities.get(7), keys.get(3), keys.get(99_999)));
    Prim.debugPrint(debug_show (sameCounts.get("Bern"), sameCounts.get("Chur")));
    // Every row read through each view equals the fields of the row read whole
    var same = 0;
    var i = 0;
    while i < 4_200 {
      switch (orders.get(i), names.get(i), money.get(i), cities.get(i), keys.get(i), all.get(i)) {
        case (?o, ?n, ?m, ?c, ?{}, ?a) {
          if n == ({ customer = o.customer; city = o.city }) and m == ({ amount = o.amount; note = o.note }) and c.city == o.city and a == o {
            same += 1
          }
        }
        case _ {}
      };
      i += 1
    };
    Prim.debugPrint(debug_show same)
  };

  // The number of values two iterators give alike, in order; 0 when one gives more
  func alike<A, B>(a : { next : () -> ?A }, b : { next : () -> ?B }, eq : (A, B) -> Bool) : Nat {
    var n = 0;
    loop {
      switch (a.next(), b.next()) {
        case (null, null) { return n }
        case (?x, ?y) {
          if not eq(x, y) { return 0 };
          n += 1
        }
        case _ { return 0 }
      }
    }
  };

  func name(o : Order) : Name = { customer = o.customer; city = o.city };

  func nameEntry((k, o) : (Nat, Order), (k2, n) : (Nat, Name)) : Bool = k == k2 and n == name(o);

  // Every read of rows through a view gives the rows the table gives, with the view's fields
  public query func reads() : async () {
    let basel = ?[#text("Basel")];
    let amounts = #range { low = ?#incl(#nat(700)); high = ?#excl(#nat(1_400)) };
    Prim.debugPrint(
      debug_show (
        alike(orders.entries(), names.entries(), nameEntry),
        alike(orders.keys(), names.keys(), func(a : Nat, b : Nat) : Bool = a == b),
        alike(orders.keys(), keys.entries(), func(a : Nat, (b, _) : (Nat, {})) : Bool = a == b),
        alike(orders.values(), money.values(), func(o : Order, m : Money) : Bool = m == ({ amount = o.amount; note = o.note })),
        alike(orders.entries(), all.entries(), func(a : (Nat, Order), b : (Nat, Order)) : Bool = a == b),
        alike(orders.indexEntries(["city"], basel, basel), names.indexEntries(["city"], basel, basel), nameEntry),
        alike(orders.reverseIndexEntries(["city"], null, null), names.reverseIndexEntries(["city"], null, null), nameEntry),
        alike(orders.entriesWhere("amount", amounts), names.entriesWhere("amount", amounts), nameEntry),
      )
    );
    Prim.debugPrint(debug_show (names.minEntry(["city"]), cities.maxEntry(["city"])));
    Prim.debugPrint(debug_show (sameCounts.entries().next(), sameCounts.values().next()))
  };

  // The calls that name fields name any field of the table on a view
  public query func named() : async () {
    let paid = ?("paid", #eq(#bool(true)));
    Prim.debugPrint(
      debug_show (
        alike(orders.cells(true, ["placed", "city"], paid), names.cells(true, ["placed", "city"], paid), func(a : [NativeTable.Cell], b : [NativeTable.Cell]) : Bool = a == b),
        alike(orders.groupCounts(["city"]), names.groupCounts(["city"]), func(a : ([NativeTable.Cell], Nat), b : ([NativeTable.Cell], Nat)) : Bool = a == b),
        names.size() == orders.size(),
        names.count(["city"], [#text("Bern")]) == orders.count(["city"], [#text("Bern")]),
        names.countWhere("paid", #eq(#bool(true))) == orders.countWhere("paid", #eq(#bool(true))),
        names.sumCount("amount") == orders.sumCount("amount"),
        names.sumWhere("amount", "city", #eq(#text("Basel"))) == orders.sumWhere("amount", "city", #eq(#text("Basel"))),
        (names.min("placed"), names.max("amount")) == (orders.min("placed"), orders.max("amount")),
        names.fieldTypes() == orders.fieldTypes(),
      )
    );
    Prim.debugPrint(debug_show (names.count(["city"], [#text("Bern")]), names.min("placed"), names.size()))
  };

  // Delete and remove on a view remove the whole row; maintain on a view runs the table's upkeep
  public func removeOnView() : async () {
    Prim.debugPrint(debug_show (names.delete(3), names.delete(3), orders.get(3), money.get(3)));
    names.remove(4_199);
    Prim.debugPrint(debug_show (orders.get(4_199), names.size(), names.maintain(1_000_000)))
  };

  // A field is added and one dropped; the view of the changed table reads both kinds of rows
  public func change() : async () {
    let t : NativeTable.NativeTable<Nat, New> = people.evolve();
    t.add(3, { name = "Cy"; age = 20; email = ?"cy@example.org" });
    changed := ?t
  };

  public query func getChanged() : async () {
    switch changed {
      case ?t {
        let v : NativeTable.NativeTable<Nat, { email : ?Text; name : Text }> = t.view();
        Prim.debugPrint(debug_show (v.get(1), v.get(3), v.get(4)));
        let rows = v.entries();
        Prim.debugPrint(debug_show (rows.next(), rows.next(), rows.next(), rows.next()));
        Prim.debugPrint(debug_show (v.cells(false, ["age"], null).next(), v.sum("age"), v.size()))
      }
      case null {}
    }
  };

  // A view made before the row type changed
  public query func getOld() : async () {
    Prim.debugPrint(debug_show (oldNames.get(1)))
  };

  // A scan through a view made before the row type changed
  public query func scanOld() : async () {
    Prim.debugPrint(debug_show (oldNames.entries().next()))
  };

  public func missingField() : async () {
    let v : NativeTable.NativeTable<Nat, { customer : Text; town : Text }> = orders.view();
    ignore v
  };

  public func otherType() : async () {
    let v : NativeTable.NativeTable<Nat, { amount : Int }> = orders.view();
    ignore v
  };

  public func notOption() : async () {
    let v : NativeTable.NativeTable<Nat, { note : Text }> = orders.view();
    ignore v
  };

  // The keys of the rows whose city is `city`, in the index's order
  func inCity(city : Text) : [Nat] {
    let ks = orders.indexEntries(["city"], ?[#text(city)], ?[#text(city)]);
    let out = Prim.Array_init<Nat>(orders.count(["city"], [#text(city)]), 0);
    var n = 0;
    for (k, _) in ks { out[n] := k; n += 1 };
    Prim.Array_tabulate<Nat>(n, func i = out[i])
  };

  public func updates() : async () {
    let (sum, rows) = switch orders.sumCount("amount") { case ?s { s } case null { (0, 0) } };
    Prim.debugPrint(
      debug_show (
        orders.update(9, { city = "Chur" }),
        orders.update(8, { amount = 5; paid = false }),
        orders.update(4_150, { note = ?"kept" }),
        orders.update(4_160, { city = "Chur"; placed = 5 }),
        orders.update(10, { placed = -10 }),
        orders.update(99_999, { paid = true }),
      )
    );
    Prim.debugPrint(debug_show (inCity("Chur"), orders.sumCount("amount") == ?(sum - 2 ** 20_000 + 5, rows), orders.max("placed"), orders.size()));
    Prim.debugPrint(
      debug_show (
        scores.update(1, { score = 5 }),
        scores.update(1, { best = ?7 }),
        scores.get(1),
        scores.update(1, { best = null : ?Int; tag = "b" }),
        scores.get(1),
        scores.count(["score"], [#int(5)]),
      )
    );
    Prim.debugPrint(
      debug_show (
        scores.upsert(2, { score = 4; tag = "c" }),
        scores.get(2),
        scores.upsert(2, { score = 6; best = ?1; tag = "d" }),
        scores.upsert(2, { score = 6; tag = "e" }),
        scores.get(2),
        scores.count(["score"], [#int(6)]),
        scores.count(["score"], [#int(4)]),
      )
    );
    switch changed {
      case ?t {
        Prim.debugPrint(debug_show (t.update(1, { email = ?"ana@example.org" }), t.update(2, { age = 31 }), t.get(1), t.get(2)))
      }
      case null {}
    }
  };

  // The updated rows read whole, each against the row it was given with the updated fields
  public query func getUpdated() : async () {
    Prim.debugPrint(
      debug_show (
        orders.get(9) == ?{ order(9) with city = "Chur" },
        orders.get(8) == ?{ order(8) with customer = long(); amount = 5; paid = false },
        orders.get(4_150) == ?{ order(4_150) with customer = long(); amount = 2 ** 460_000; note = ?"kept" },
        orders.get(4_160) == ?{ order(4_160) with city = "Chur"; placed = 5 },
        orders.get(10) == ?order(10),
        orders.get(99_999),
        names.get(9),
      )
    )
  };

  public func updateMissing() : async () {
    ignore orders.update(9, { town = "Basel" })
  };

  public func updateOtherType() : async () {
    ignore orders.update(9, { paid = 1 })
  };

  public func updateIntForNat() : async () {
    ignore orders.update(9, { amount = -1 })
  };

  public func updateOnView() : async () {
    ignore names.update(9, { city = "Basel" })
  };

  public func addOnView() : async () {
    all.add(1, order(1))
  };

  public func evolveView() : async () {
    let v : NativeTable.NativeTable<Nat, { customer : Text }> = all.evolve();
    ignore v
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress reads "DIDL\x00\x00"
//CALL ingress named "DIDL\x00\x00"
//CALL ingress getOld "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress getChanged "DIDL\x00\x00"
//CALL ingress getOld "DIDL\x00\x00"
//CALL ingress scanOld "DIDL\x00\x00"
//CALL ingress missingField "DIDL\x00\x00"
//CALL ingress otherType "DIDL\x00\x00"
//CALL ingress notOption "DIDL\x00\x00"
//CALL ingress addOnView "DIDL\x00\x00"
//CALL ingress evolveView "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress removeOnView "DIDL\x00\x00"
//CALL ingress reads "DIDL\x00\x00"
//CALL ingress named "DIDL\x00\x00"
//CALL ingress updates "DIDL\x00\x00"
//CALL ingress getUpdated "DIDL\x00\x00"
//CALL ingress updateMissing "DIDL\x00\x00"
//CALL ingress updateOtherType "DIDL\x00\x00"
//CALL ingress updateIntForNat "DIDL\x00\x00"
//CALL ingress updateOnView "DIDL\x00\x00"
//CALL ingress getUpdated "DIDL\x00\x00"
