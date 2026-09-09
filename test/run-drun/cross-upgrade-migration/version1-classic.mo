// Provides provenance for the committed classical `old-v1.wasm` fixture used by
// `cross-upgrade-migration.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";
import Migration "Migration1";

// test merging fields `one` and `two` into field `three`, dropping `one` and `two` and preserving `zero`.
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 1");

   stable var zero : Nat = 0;

   stable var three : [var (Nat, Text)] = [var (1,"1")];

   public func check(): async() {
     Prim.debugPrint (debug_show {zero; three});
   }
};
