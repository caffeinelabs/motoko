//MOC-FLAG -fshared-code
func goNat32() {
 let x : Nat32 = 1 +% 1;
 var y : Nat32 = 1; y *%= 2;
 assert (x *% y == 4);
};
goNat32();


func goInt32() {
 let x : Int32 = 1 +% 1;
 var y : Int32 = 1; y *%= 2;
 assert (x *% y == 4);
};
goInt32();

func goNat64() {
 let x : Nat64 = 1 +% 1;
 var y : Nat64 = 1; y *%= 2;
 assert (x *% y == 4);
};
goNat64();

func goInt64() {
 let x : Int64 = 1 +% 1;
 var y : Int64 = 1; y *%= 2;
 assert (x *% y == 4);
};
goInt64();

func goFloat() {
 let x : Float = 1 + 1;
 var y : Float = 1; y *= 2;
 assert (x * y == 4);
};
goFloat();

// This just checks that the above CHECK-NOT: box_i64 is up-to-date
// (If that function gets renamed the test above could yield false successes
func goValidNat64(x : Nat64) { assert (x *% x == 4); }; goValidNat64(2);
func goValidFloat(x : Float) { assert (x * x == 4); }; goValidFloat(2);

// We have to do the filechecks in reverse order:

// CHECK: func $goValidFloat
// CHECK: f64.load
// CHECK: unreachable

// CHECK: func $goValidNat64
// CHECK: unbox64
// CHECK: unreachable

// CHECK: func $goFloat
// CHECK-NOT: f64.load
// CHECK-STORE: f64.load
// CHECK: unreachable

// CHECK: func $goInt64
// CHECK-NOT: box64
// CHECK-NOT: unbox64
// CHECK: unreachable

// CHECK: func $goNat64
// CHECK-NOT: box64
// CHECK-NOT: unbox64
// CHECK: unreachable


// Also test that the value is included in a closure properly

func goCaptureNat64() : () -> () {
 let x : Nat64 = 1 +% 1;
 var y : Nat64 = 1 +% 1;
 return func() { assert(x == 2); assert(y == 2)};
};
goCaptureNat64()();

func goCaptureInt64() : () -> () {
 let x : Int64 = 1 +% 1;
 var y : Int64 = 1 +% 1;
 return func() { assert(x == 2); assert(y == 2)};
};
goCaptureInt64()();

func goCaptureFloat() : () -> () {
 let x : Float = 1 + 1;
 var y : Float = 1 + 1;
 return func() { assert(x == 2); assert(y == 2)};
};
goCaptureFloat()();
