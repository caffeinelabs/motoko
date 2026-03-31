import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

let pi32 : Float32 = 3.14159265358979323846;
let x : Float32 = 41.99;
let result : Float32 = x % pi32;
debugPrint (debug_show (float32ToFloat result));

// chase_forwarders: call site jumps directly to the libm implementation, bypassing $fmodf
//CHECK: f32.const 0x1.4feb86p+5
//CHECK-NEXT: f32.const 0x1.921fb6p+1
//CHECK-NEXT: call $libm{{.*fmodf.*}}
// The $fmodf wrapper is still present in the binary (unreferenced; DCE'd by wasm-opt)
//CHECK: (func $fmodf
//CHECK-NEXT: local.get 0
//CHECK-NEXT: local.get 1
//CHECK-NEXT: call $libm{{.*}}){{$}}

//SKIP run-ir
//SKIP run-low
