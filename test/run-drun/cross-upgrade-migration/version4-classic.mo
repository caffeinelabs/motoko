import Prim "mo:prim";
import Migration "Migration4";

// test adding a nested field, changing type
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 4");

   var zero : Nat = 0;

   var four : [var (Text, Nat, Bool)] = [var ("1", 1, false)];

   public func check(): async() {
     Prim.debugPrint(debug_show{zero; four});
   }
};
