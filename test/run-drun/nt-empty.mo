import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Two empty tables keep their size across an upgrade
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().build();
  let flags = NativeTable.empty<Text, Bool>().build();

  public query func check() : async () {
    Prim.debugPrint(debug_show (users.size(), users.isEmpty(), flags.size(), flags.isEmpty()))
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress check "DIDL\x00\x00"
//CALL upgrade
//CALL ingress check "DIDL\x00\x00"
