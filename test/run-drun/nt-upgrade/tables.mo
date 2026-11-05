import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Rows, handles and indexes survive upgrades that keep main memory, and writes keep working
// after them. Keys 0 to 1,999 get rows first; after the upgrade, keys below 2,000 with remainder 0
// by 3 are removed, those with remainder 1 get the next key's row, and keys 2,000 to 2,999 get
// rows. The index on city counts the rows of each city, and walks them by city.
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().createIndex(["city"]).build();
  let places = NativeTable.empty<Text, Text>().build();
  // A handle made by a message and kept in a variable
  var counts : ?NativeTable.NativeTable<Nat, Nat> = null;

  func user(i : Nat) : User {
    let city = switch i % 3 { case 0 { "Basel" } case 1 { "Bern" } case _ { "Chur" } };
    { name = "user " # debug_show i; city; amount = i * i }
  };

  // A Text of 131,072 bytes, spread over more than one page of table memory
  func long() : Text {
    var t = "Zürich ";
    var i = 0;
    while i < 14 { t := t # t; i += 1 };
    t
  };

  public func put() : async () {
    var i = 0;
    while i < 2_000 { users.add(i, user(i)); i += 1 };
    users.add(5_000, { name = "Gil"; city = long(); amount = 2 ** 20_000 });
    places.add("Zürich", "on the Limmat");
    places.add("", "");
    let c = NativeTable.empty<Nat, Nat>().build();
    c.add(1, 2 ** 64);
    counts := ?c
  };

  public func write() : async () {
    var i = 0;
    while i < 3_000 {
      if i >= 2_000 {
        assert users.insert(i, user(i))
      } else if i % 3 == 0 {
        assert users.delete(i)
      } else if i % 3 == 1 {
        assert not users.insert(i, user(i + 1))
      };
      i += 1
    };
    places.add("Genève", "on the Rhône");
    switch counts {
      case ?c { c.add(2, 2 ** 65) }
      case null { assert false }
    }
  };

  public query func check() : async () {
    // Keys 0 to 2,999: rows equal to their key's row, to the next key's row, or missing
    var same = 0;
    var next = 0;
    var missing = 0;
    var i = 0;
    while i < 3_000 {
      let r = users.get(i);
      if r == ?user(i) { same += 1 }
      else if r == ?user(i + 1) { next += 1 }
      else if r == null { missing += 1 };
      i += 1
    };
    Prim.debugPrint(debug_show { size = users.size(); same; next; missing });
    Prim.debugPrint(debug_show (users.get(1), users.get(2_999)));
    let city = ["city"];
    var walked = 0;
    var last = "";
    for (_, u) in users.indexEntries(city, null, null) {
      if u.city < last { Prim.debugPrint("walk out of order") };
      last := u.city;
      walked += 1
    };
    Prim.debugPrint(debug_show { indexes = users.indexes; Basel = users.count(city, [#text "Basel"]); Bern = users.count(city, [#text "Bern"]); Chur = users.count(city, [#text "Chur"]); walked });
    switch users.get(5_000) {
      case ?gil { Prim.debugPrint(debug_show (gil.name, gil.city == long(), gil.amount == 2 ** 20_000)) }
      case null { Prim.debugPrint("long row missing") }
    };
    Prim.debugPrint(debug_show (places.size(), places.get("Zürich"), places.get(""), places.get("Genève")));
    switch counts {
      case ?c { Prim.debugPrint(debug_show (c.size(), c.get(1), c.get(2))) }
      case null { Prim.debugPrint("no counts table") }
    }
  }
}

//MOC-FLAG --package native-table ../../lib/native-table
