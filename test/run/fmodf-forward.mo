import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

let pi32 : Float32 = 3.14159265358979323846;
let x : Float32 = 41.99;
let result : Float32 = x % pi32;
debugPrint (debug_show (float32ToFloat result));

// Motoko-level forwarding chain: foo -> bar -> quux (3 differently-typed args)
func quux(a : Float32, b : Float, _c : Nat32) : Float =
  float32ToFloat a + b;
func bar(a : Float32, b : Float, c : Nat32) : Float = quux (a, b, c);
func foo(a : Float32, b : Float, c : Nat32) : Float = bar (a, b, c);
// baz calls quux but adds 1.0 — NOT a pure forwarder
func baz(a : Float32, b : Float, c : Nat32) : Float = quux (a, b, c) + 1.0;

let _ = foo (floatToFloat32 1.0, 2.0, 3);
let _ = baz (floatToFloat32 1.0, 2.0, 3);

// chase_forwarders: call site jumps directly to the libm implementation, bypassing $fmodf
//CHECK: f32.const 0x1.4feb86p+5
//CHECK-NEXT: f32.const 0x1.921fb6p+1
//CHECK-NEXT: call $libm{{.*fmodf.*}}
// baz has extra work after call $quux (unbox + f64.add) — not a forwarder
//CHECK: (func $baz
//CHECK: call $quux
//CHECK-NEXT: f64.load
// foo is NOT chased: its body passes i32.const 0 (not local.get $clos) as closure arg
//CHECK: (func $foo
//CHECK-NEXT: i32.const 0
//CHECK: call $bar)
// bar likewise
//CHECK: (func $bar
//CHECK-NEXT: i32.const 0
//CHECK: call $quux)
// The $fmodf wrapper is still present in the binary (unreferenced; DCE'd by wasm-opt)
//CHECK: (func $fmodf
//CHECK-NEXT: local.get 0
//CHECK-NEXT: local.get 1
//CHECK-NEXT: call $libm{{.*}}){{$}}

//SKIP run-ir
//SKIP run-low
