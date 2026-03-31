import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

let pi32 : Float32 = 3.14159265358979323846;
let x : Float32 = 41.99;
let result : Float32 = x % pi32;
debugPrint (debug_show (float32ToFloat result));

// Status quo: call site goes through the $fmodf wrapper (two-hop indirection)
//CHECK: f32.const 0x1.4feb86p+5
//CHECK-NEXT: f32.const 0x1.921fb6p+1
//CHECK-NEXT: call $fmodf
// The $fmodf wrapper is a pure forwarder: local.get 0; local.get 1; call $libm...fmodf...) — end of body
//CHECK: (func $fmodf
//CHECK-NEXT: local.get 0
//CHECK-NEXT: local.get 1
//CHECK-NEXT: call $libm{{.*}}){{$}}

//SKIP run-ir
//SKIP run-low
