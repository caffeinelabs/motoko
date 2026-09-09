// Provides provenance for the committed classical `old-v2.wasm` fixture used by
// `cross-upgrade-migration.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";
import Migration "Migration2";

// Rename stable field `three` to `four`
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 2");

   stable var zero : Nat = 0; // inherited

   stable var four : [var (Nat, Text)] = [var (1, "1")];

   public func check(): async() {
     Prim.debugPrint(debug_show{zero; four});
   }
};
