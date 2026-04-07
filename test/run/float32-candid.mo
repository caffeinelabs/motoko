import { debugPrint } = "mo:⛔";

// Float32 literal round-trip through Candid
let blob = to_candid (3.14 : Float32);
debugPrint (debug_show blob);
let back : ?Float32 = from_candid blob;
assert (back == ?(3.14 : Float32));

// Exact value (1.5 is exactly representable)
let blob2 = to_candid (1.5 : Float32);
debugPrint (debug_show blob2);
let back2 : ?Float32 = from_candid blob2;
assert (back2 == ?(1.5 : Float32));

// from_candid of an f64 blob yields null (type mismatch)
let f64blob = to_candid (1.5 : Float);
let mismatch : ?Float32 = from_candid f64blob;
assert (mismatch == null);

//SKIP run
//SKIP run-ir
//SKIP run-low
