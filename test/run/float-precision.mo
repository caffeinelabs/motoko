// M0266 (shared by Float32 and Float): a floating-point literal with more
// significant digits than the type can hold warns (and suggests the shortest
// round-trip form); minimal literals stay quiet. For Float both the
// `: Float`-annotated site and the unannotated (inferred, defaults to Float)
// site are covered.
let excess : Float32 = 0.123456789;  // warns: rounds to 0.12345679
let minimal : Float32 = 0.1;         // no warning
let exact : Float32 = 1.0;           // no warning
let excessF : Float = 0.12345678901234567890;  // warns (annotated : Float)
let inferredF = 0.12345678901234567890;         // warns (unannotated, defaults to Float)
let minimalF : Float = 0.1;                      // no warning
let exactF = 1.0;                                // no warning (unannotated)
assert (excess != minimal and minimal != exact);
assert (excessF != minimalF and minimalF != exactF and inferredF == excessF);
