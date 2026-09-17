import Prim "mo:prim";
import Migration "Migration1";

// test merging fields `one` and `two` into field `three`, dropping `one` and `two` and preserving `zero`.
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 1");

   var zero : Nat = 0;

   var three : [var (Nat, Text)] = [var (1,"1")];

   public func check(): async() {
     Prim.debugPrint (debug_show {zero; three});
   }
};
