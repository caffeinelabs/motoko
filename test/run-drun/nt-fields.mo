import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A view reads some fields of each row: its get gives the record of the view's fields, equal to
// those fields of the row get gives, for rows still in the open group, packed rows and replaced
// packed rows, long values, a table whose row type changed, a view of a view, a view of no field
// and a view of a scalar row. A view of a field the row lacks or has with another type, add and
// evolve on a view, and a view made before an evolve trap, and change nothing
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
        Prim.debugPrint(debug_show (v.get(1), v.get(3), v.get(4)))
      }
      case null {}
    }
  };

  // A view made before the row type changed
  public query func getOld() : async () {
    Prim.debugPrint(debug_show (oldNames.get(1)))
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
//CALL ingress getOld "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress getChanged "DIDL\x00\x00"
//CALL ingress getOld "DIDL\x00\x00"
//CALL ingress missingField "DIDL\x00\x00"
//CALL ingress otherType "DIDL\x00\x00"
//CALL ingress notOption "DIDL\x00\x00"
//CALL ingress addOnView "DIDL\x00\x00"
//CALL ingress evolveView "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
