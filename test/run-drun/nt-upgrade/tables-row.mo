import NativeTable "mo:native-table/NativeTable";

// The stable variables of `tables.mo`, with one more field, an option, in the row type of `users`:
// a table's key and row types cannot change, so the upgrade is refused
actor {
  type User = { name : Text; city : Text; amount : Nat; extra : ?Nat };

  let users = NativeTable.empty<Nat, User>().createIndex(["city"]).build();
  let places = NativeTable.empty<Text, Text>().build();
  let none = NativeTable.empty<Text, Bool>().build();
  var counts : ?NativeTable.NativeTable<Nat, Nat> = null
}

//MOC-FLAG --package native-table ../../lib/native-table
