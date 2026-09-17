import Prim "mo:prim";
import Migration "Migration2";

// Rename stable field `three` to `four`
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 2");

   var zero : Nat = 0; // inherited

   var four : [var (Nat, Text)] = [var (1, "1")];

   public func check(): async() {
     Prim.debugPrint(debug_show{zero; four});
   }
};
