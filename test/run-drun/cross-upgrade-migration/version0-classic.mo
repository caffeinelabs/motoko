// Provides provenance for the committed classical `old-v0.wasm` fixture used by
// `cross-upgrade-migration.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";

actor {
   Prim.debugPrint("Version 0");

   stable var zero : Nat = 0;

   stable var one : [var Nat] = [var 1];
   stable var two : [var Text] = [var "1"];

   public func check(): async() {
     Prim.debugPrint (debug_show {zero;one;two})
   };
};
