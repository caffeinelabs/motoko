import Prim "mo:prim";

actor {
   Prim.debugPrint("Version 0");

   stable var zero : Nat = 0;

   stable var one : [var Nat] = [var 1, 2, 3, 4];
   stable var two : [var Text] = [var "1", "2", "3", "4"];

   public func check(): async() {
     Prim.debugPrint (debug_show {zero;one;two})
   };
};
