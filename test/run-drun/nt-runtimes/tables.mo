import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A version with the stable variables of `plain.mo` that makes a table, so it links the runtime
// with native tables. It keeps the table's id without its codec, and gets the codec back from the
// compiler for each call.
actor {
  type User = { name : Text; city : Text; amount : Nat };

  var users : [(Nat, User)] = [];
  var kept : ?{ id : Nat64; indexes : [([Text], Nat)] } = null;

  func table() : NativeTable.NativeTable<Nat, User> {
    switch kept {
      case ?k { { k with codec = NativeTable.empty<Nat, User>().codec } }
      case null { Prim.trap("no table") }
    }
  };

  public func makeTable() : async () {
    let t = NativeTable.empty<Nat, User>().createIndex(["city"]).build();
    var i = 0;
    while i < 1_000 {
      t.add(i, { name = "user " # debug_show i; city = if i % 2 == 0 { "Basel" } else { "Bern" }; amount = i });
      i += 1
    };
    kept := ?{ id = t.id; indexes = t.indexes }
  };

  public func write() : async () {
    table().add(1_000, { name = "Ed"; city = "Ems"; amount = 2 ** 100 })
  };

  public query func check() : async () {
    let t = table();
    Prim.debugPrint(debug_show (users, t.size(), t.get(1), t.get(1_000), t.count(["city"], [#text "Basel"])))
  }
}

//MOC-FLAG --package native-table ../../lib/native-table
