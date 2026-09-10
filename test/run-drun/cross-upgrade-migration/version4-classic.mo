// Provides provenance for the committed classical `old-v4.wasm` fixture used by
// `cross-upgrade-migration.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";
import Migration "Migration4";

// test adding a nested field, changing type
(with migration = Migration.run)
actor {

   Prim.debugPrint("Version 4");

   stable var zero : Nat = 0;

   stable var four : [var (Text, Nat, Bool)] = [var ("1", 1, false)];

   public func check(): async() {
     Prim.debugPrint(debug_show{zero; four});
   }
};
