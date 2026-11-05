import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Writes report whether the key is new, and the sizes count each key once
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().build();
  let places = NativeTable.empty<Text, Text>().build();

  public func put() : async () {
    // New keys, small and big
    Prim.debugPrint(debug_show users.insert(7, { name = "Ana"; city = "Basel"; amount = 40 }));
    Prim.debugPrint(debug_show users.insert(2 ** 62, { name = "Bo"; city = "Bern"; amount = 2 ** 100 }));
    Prim.debugPrint(debug_show users.insert(2 ** 100, { name = "Cy"; city = "Chur"; amount = 0 }));
    users.add(8, { name = "Di"; city = "Davos"; amount = 2 ** 64 });
    // A record with more fields than the row type
    let wider = { name = "Ed"; city = "Ems"; amount = 5; active = true; var visits = 1 };
    Prim.debugPrint(debug_show users.insert(9, wider));
    Prim.debugPrint(debug_show users.size());

    // The same keys again replace their rows
    Prim.debugPrint(debug_show users.insert(7, { name = "Ana"; city = "Zug"; amount = 41 }));
    Prim.debugPrint(debug_show users.insert(2 ** 62, { name = "Bo"; city = "Bern"; amount = 1 }));
    Prim.debugPrint(debug_show users.insert(2 ** 100, { name = "Cy"; city = "Chur"; amount = 2 ** 70 }));
    users.add(8, { name = "Di"; city = "Davos"; amount = 2 });
    Prim.debugPrint(debug_show users.size());

    // Text keys and rows: concatenated, non-ASCII and empty
    let key = "Zürich, " # "the largest city of Switzerland";
    Prim.debugPrint(debug_show places.insert(key, "on the " # "Limmat"));
    Prim.debugPrint(debug_show places.insert("Genève", "on the Rhône"));
    Prim.debugPrint(debug_show places.insert("", ""));
    Prim.debugPrint(debug_show places.insert("Zürich, the largest city of Switzerland", "on the Sihl"));
    Prim.debugPrint(debug_show places.insert("", "nowhere"));
    Prim.debugPrint(debug_show places.size())
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show (users.size(), places.size()))
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress check "DIDL\x00\x00"
//CALL ingress put "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
