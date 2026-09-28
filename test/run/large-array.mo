import Prim "mo:⛔";

// Should allocate 1G
ignore(Prim.Array_init<()>(1024 * 1024 * 1024 / 4, ()));

Prim.debugPrint "done";

//SKIP run
//SKIP run-ir
//SKIP run-low
