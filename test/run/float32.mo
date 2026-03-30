import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

// 1.5 is exactly representable in f32, round-trips losslessly
let f : Float = 1.5;
let f32 : Float32 = floatToFloat32 f;
let back : Float = float32ToFloat f32;
assert back == 1.5;

// A value with excess f64 precision that gets truncated by f32
// 0.1 in f64: 0.1000000000000000055511151231257827021181583404541015625
// 0.1 in f32 (back to f64): 0.100000001490116119384765625
let f64precise : Float = 0.1;
let f32truncated : Float32 = floatToFloat32 f64precise;
let f32back : Float = float32ToFloat f32truncated;
// After round-trip through f32 the value must differ from the f64 original
assert f32back != f64precise;
// But round-tripping through f32 twice is idempotent
assert float32ToFloat (floatToFloat32 f32back) == f32back;

debugPrint ("Float32 precision tests passed: " # debug_show f32);

// Float32 literal ascription
let lit32 : Float32 = 3.14;
assert debug_show lit32 == debug_show (floatToFloat32 3.14);

// Arithmetic operations (via Float conversions)
let a : Float32 = floatToFloat32 2.0;
let b : Float32 = floatToFloat32 3.0;

assert float32ToFloat (a + b) == 5.0;
assert float32ToFloat (b - a) == 1.0;
assert float32ToFloat (a * b) == 6.0;
assert float32ToFloat (b / a) == 1.5;
assert float32ToFloat (b % a) == 1.0;
assert float32ToFloat (-a)   == -2.0;

// pow: exact case (2^10 = 1024)
let ten : Float32 = floatToFloat32 10.0;
assert float32ToFloat (a ** ten) == 1024.0;

// pow: irrational case (sqrt(2) = 2^0.5); use epsilon
let half : Float32 = floatToFloat32 0.5;
let sqrt2f32 : Float = float32ToFloat (a ** half);
let sqrt2f64 : Float = float32ToFloat (floatToFloat32 1.4142135623730951);
let diff : Float = sqrt2f32 - sqrt2f64;
assert diff * diff < 1e-10;

// Comparisons
assert a == a;
assert not (a == b);
assert a != b;
assert a <  b;
assert a <= a;
assert b >  a;
assert b >= b;

// Arithmetic operations — literal style (no Float conversions)
let a2 : Float32 = 2.0;
let b2 : Float32 = 3.0;

assert a2 + b2 == 5.0;
assert b2 - a2 == 1.0;
assert a2 * b2 == 6.0;
assert b2 / a2 == 1.5;
assert b2 % a2 == 1.0;
assert -a2     == -2.0;

// pow: exact case (2^10 = 1024)
assert a2 ** 10.0 == 1024.0;

// pow: sqrt(2) = 2^0.5; epsilon stays in Float32
let sqrt2 : Float32 = a2 ** 0.5;
let diffL : Float32 = sqrt2 - 1.4142135623730951;
assert diffL * diffL < 1e-10;

// Comparisons
assert a2 == a2;
assert not (a2 == b2);
assert a2 != b2;
assert a2 <  b2;
assert a2 <= a2;
assert b2 >  a2;
assert b2 >= b2;

//MOC-FLAG -dl
//FILTER comp grep -e Float32Lit -e passed
//FILTER run grep -e Float32Lit -e passed
//FILTER run-ir grep -e Float32Lit -e passed
//FILTER run-low grep -e Float32Lit -e passed
