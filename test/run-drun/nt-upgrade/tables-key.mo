import NativeTable "mo:native-table/NativeTable";

// The stable variables of `tables.mo`, with key type Nat instead of Text for `places`: a table's
// key and row types cannot change, so the upgrade is refused
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().createIndex(["city"]).build();
  let places = NativeTable.empty<Nat, Text>().build();
  var counts : ?NativeTable.NativeTable<Nat, Nat> = null
}

//MOC-FLAG --package native-table ../../lib/native-table
