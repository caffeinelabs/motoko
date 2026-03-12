import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

// 1.5 is exactly representable in f32, round-trips losslessly
let f : Float = 1.5;
let f32 : Float32 = floatToFloat32 f;
let back : Float = float32ToFloat f32;
assert (back == 1.5);

// A value with excess f64 precision that gets truncated by f32
// 0.1 in f64: 0.1000000000000000055511151231257827021181583404541015625
// 0.1 in f32 (back to f64): 0.100000001490116119384765625
let f64precise : Float = 0.1;
let f32truncated : Float32 = floatToFloat32 f64precise;
let f32back : Float = float32ToFloat f32truncated;
// After round-trip through f32 the value must differ from the f64 original
assert (f32back != f64precise);
// But round-tripping through f32 twice is idempotent
assert (float32ToFloat (floatToFloat32 f32back) == f32back);

debugPrint (debug_show f32);
debugPrint "Float32 precision tests passed";

//MOC_FLAG -dp
//SKIP comp
