//MOC-FLAG --enhanced-orthogonal-persistence
import Prim "mo:prim";

// drops the migration expression from version4.mo
actor {

   Prim.debugPrint("Version 5");

   var zero : Nat = Prim.trap "unreachable"; // inherited
   assert zero == 0;

   var four : [var (Text, Nat, Bool)] = [var];

   public func check(): async() {
     Prim.debugPrint(debug_show{zero; four});
   }
};
