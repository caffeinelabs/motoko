// Provides provenance for the committed classical `old.wasm` fixture used by
// `migrate-candid-sharing.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";

actor {
   type Data = { field1 : Text; field2 : Nat; var field3: ?Data; };

   stable var sharedObject : Data = { field1 = "Test"; field2 = 12345; var field3 = null };
   sharedObject.field3 := ?sharedObject;

   stable var array : [var Data] = Prim.Array_init<Data>(100, sharedObject);

   Prim.debugPrint("INITIALIZED: " # debug_show (array.size()));
};
