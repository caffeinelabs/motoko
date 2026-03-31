import { floatToFloat32; float32ToFloat; debugPrint } = "mo:⛔";

let pi32 : Float32 = 3.14159265358979323846;
let x : Float32 = 41.99;
let result : Float32 = x % pi32;
debugPrint (debug_show (float32ToFloat result));

// Nested closure chain: foo_clos -> bar -> quux (quux sunk into bar; bar is a forwarder)
func foo_clos(a : Float32, b : Float, c : Nat32) : Float {
  func bar(a : Float32, b : Float, c : Nat32) : Float {
    func quux(a : Float32, b : Float, _ : Nat32) : Float = if (c != 0) (float32ToFloat a + b) else b;
    quux(a, b, c)  // bar forwards all its args — no capture of c
  };
  bar(a, b, c)  // foo_clos computes c+c and passes it; bar need not capture c
};

let _ = foo_clos (floatToFloat32 1.0, 2.0, 3);

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
// foo→bar→quux chain collapsed: foo now calls quux directly (bar was a 0-forwarder)
//CHECK: (func $foo
//CHECK-NEXT: i32.const 0
//CHECK: call $quux)
// top-level bar: 0-forwarder body unchanged; unreferenced after chase (DCE'd by wasm-opt)
//CHECK: (func $bar
//CHECK-NEXT: i32.const 0
//CHECK: call $quux)
// foo_clos is also a 0-forwarder: delegates entirely to nested bar.1
//CHECK: (func $foo_clos
//CHECK-NEXT: i32.const 0
//CHECK: call $bar.1)
// nested bar (bar.1): builds quux.1's closure (stores c at offset 13), dispatches via call_indirect
//CHECK: (func $bar.1
//CHECK: i32.store offset=13
//CHECK: call_indirect
// quux.1 (inner quux): loads captured c from $clos — real closure, NOT a 0-forwarder
//CHECK: (func $quux.1
//CHECK: local.get $clos
// The $fmodf wrapper is still present in the binary (unreferenced; DCE'd by wasm-opt)
//CHECK: (func $fmodf
//CHECK-NEXT: local.get 0
//CHECK-NEXT: local.get 1
//CHECK-NEXT: call $libm{{.*}}){{$}}

//SKIP run-ir
//SKIP run-low
//SKIP-SANITY-CHECKS
//CLASSICAL-PERSISTENCE-ONLY
