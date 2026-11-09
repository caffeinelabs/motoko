import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Rows written with add and insert read back with get: they equal the rows written, print the
// same, and their fields and patterns work as on any record. insert says whether the key is new,
// and the sizes count each key once. Rows removed with remove and delete are gone, the other rows
// stay, and a removed key is new again
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().build();
  let places = NativeTable.empty<Text, Text>().build();
  let counts = NativeTable.empty<Text, Nat>().build();

  // A Text of 131,072 bytes with two-byte characters, built by concatenation
  func long() : Text {
    var t = "Zürich ";
    var i = 0;
    while i < 14 { t := t # t; i += 1 };
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
    Prim.debugPrint(debug_show (users.size(), places.size(), counts.size()))
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
    Prim.debugPrint(debug_show (places.containsKey(""), counts.containsKey("Chur")))
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
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress remove "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
