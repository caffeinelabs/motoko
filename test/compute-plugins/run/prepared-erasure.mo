//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/prepared
//MOC-FLAG --sanity-checks
//SKIP run
//SKIP run-ir
//SKIP run-low
import Prim "mo:⛔";

// Native Candid operators are not implemented by the reference interpreters.
// Widening to Any must serialize only reserved, never the prepared payload.
let erased : Any = Prim.preparedA("private prepared bytes");
let ordinary : Any = ();
let reserved : Blob = "DIDL\00\01\70";
assert to_candid(erased) == reserved;
assert to_candid(ordinary) == reserved;
