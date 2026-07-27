// M0266: a Float32 literal with more significant digits than F32 can hold
// warns (and suggests the shortest round-trip form); minimal literals do not.
let excess : Float32 = 0.123456789;  // warns: rounds to 0.12345679
let minimal : Float32 = 0.1;         // no warning
let exact : Float32 = 1.0;           // no warning
assert (excess != minimal and minimal != exact);
