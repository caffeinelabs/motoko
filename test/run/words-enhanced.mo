//MOC-FLAG -fshared-code
import Prim "mo:⛔";

// CHECK: func $init

// CHECK: i64.const 424242
assert not (424242 : Nat64 == 1);
assert not (3 : Nat64 == 0);
assert (0 : Nat64 == 0);
// CHECK: i64.const 424342
assert not (424342 : Nat64 == 1);

// CHECK: i64.const {{1823394005778432|1823395079520256}}
assert not (424542 : Nat32 == 1);
assert not (3 : Nat32 == 0);
assert (0 : Nat32 == 0);
// CHECK: i64.const {{1824682495967232|1824683569709056}}
assert not (424842 : Nat32 == 1);

func printBit(a : Bool) { Prim.debugPrint(if a "set" else "clear") };


func checkpointAlpha() { assert true; };
func checkpointBravo() { assert true; };
func checkpointCharlie() { assert true; };
func checkpointDelta() { assert true; };
func checkpointEcho() { assert true; };
func checkpointFoxtrot() { assert true; };
func checkpointGolf() { assert true; };
func checkpointHotel() { assert true; };
func checkpointIndia() { assert true; };
func checkpointJuliett() { assert true; };

// Nat64 operations
do {
    func printN64ln(w : Nat64) {
      Prim.debugPrintNat(Prim.nat64ToNat w);
    };
    func printI64ln(w : Int64) {
      Prim.debugPrintInt(Prim.int64ToInt w);
    };

    let a : Nat64 = 4567;
    let b : Nat64 = 7;
    let c : Nat64 = 8912765;
    let d : Int64 = -15;
    let e : Nat64 = 20000;

// this is the value of c
// CHECK: i64.const 142604244
// CHECK-NOT: call $box64
// CHECK: call $printN64ln
    printN64ln(c);
    printN64ln(^c);
    printN64ln(a +% c);
    printN64ln(c -% a);

// CHECK: call $checkpointAlpha
    checkpointAlpha();
// This is a native Wasm i64 multiplication, there should be no shift involved!
// CHECK-NOT: i64.shr_u
// CHECK: call $printN64ln
    printN64ln(a * b);

    printN64ln(a / b);
    printN64ln(c % a);
    printN64ln(a ** 2);

    printN64ln(a & c);
    printN64ln(a | c);
    printN64ln(a ^ c);
    printN64ln(a << b);
    printN64ln(a >> b);
    printI64ln(-5225319197819536385 >> 4); // 0b1011011101111011111011111101111111011111111011111111101111111111L == -5225319197819536385L --> 826339054743125951L
    printI64ln(d >> 3);
    printI64ln(-5225319197819536385 >> 4); // 0b1011011101111011111011111101111111011111111011111111101111111111L == -5225319197819536385L --> -326582449863721025L

    printN64ln(c <<> b);
    printN64ln(c <>> b);
    printI64ln(Prim.popcntInt64 d); // -15 = 0xfffffffffffffff1 = 0b1111_..._1111_1111_0001 (population = 61)
    printN64ln(Prim.clzNat64 e); // 20000 = 0x0000000000004e20 (leading zeros = 49)
    printN64ln(Prim.ctzNat64 e); // 20000 = 0x0000000000004e20 (trailing zeros = 5)
    printBit(Prim.btstNat64(e, 5 : Nat64)); // 20000 = 0x0000000000004e20 (result = true)
    printBit(Prim.btstNat64(e, 63 : Nat64)); // 20000 = 0x0000000000004e20 (result = false)
    printBit(Prim.btstNat64(e, 69 : Nat64)); // 20000 = 0x0000000000004e20 (mod 64, result = true)

    assert (3 : Nat64 ** (4 : Nat64) == (81 : Nat64));
    assert (3 : Nat64 ** (7 : Nat64) == (2187 : Nat64));
    assert (3 : Nat64 ** (14 : Nat64) == (4782969 : Nat64));
    assert (3 : Nat64 ** (20 : Nat64) == (3486784401 : Nat64));
};



// Nat32 operations
do {
    func printN32ln(w : Nat32) {
      Prim.debugPrintNat(Prim.nat32ToNat w);
    };
    func printI32ln(w : Int32) {
      Prim.debugPrintInt(Prim.int32ToInt w);
    };

    let a : Nat32 = 4567;
    let b : Nat32 = 7;
    let c : Nat32 = 8912765;
    let d : Int32 = -15;
    let e : Nat32 = 20000;

// CHECK: call $checkpointBravo
    checkpointBravo();
// this is the value of c
// CHECK: i64.const 38280035265675264
// CHECK-NOT: call $box64
// CHECK: call $printN32ln
    printN32ln(c);
    printN32ln(^c);
    printN32ln(a + c);
    printN32ln(c - a);

// CHECK: call $checkpointCharlie
    checkpointCharlie();
// This is a native Wasm i64 multiplication, there should be no shift involved!
// CHECK-NOT: i64.shr_u
// CHECK: call $printN32ln
    printN32ln(a * b);
    printN32ln(a / b);
    printN32ln(c % a);
    printN32ln(a ** 2);

    printN32ln(a & c);
    printN32ln(a | c);
    printN32ln(a ^ c);
    printN32ln(a << b);
    printN32ln(a >> b);
    printI32ln(-1216614433 >> 4); // 0b10110111011110111110111111011111l == -1216614433l --> 192397053l
    printI32ln(d >> 3);
    printI32ln(-1216614433 >> 4); // 0b10110111011110111110111111011111l == -1216614433l --> -76038403
    printN32ln(c <<> b);
    printN32ln(c <>> b);
    printI32ln(Prim.popcntInt32 d); // -15 = 0xfffffff1 = 0b1111_1111_1111_1111_1111_1111_1111_0001 (population = 29)
    printN32ln(Prim.clzNat32 e); // 20000 = 0x00004e20 (leading zeros = 17)
    printN32ln(Prim.ctzNat32 e); // 20000 = 0x00004e20 (trailing zeros = 5)
    printBit(Prim.btstNat32(e, 5 : Nat32)); // 20000 = 0x00004e20 (result = true)
    printBit(Prim.btstNat32(e, 31 : Nat32)); // 20000 = 0x00004e20 (result = false)
    printBit(Prim.btstNat32(e, 37 : Nat32)); // 20000 = 0x00004e20 (mod 32, result = true)

    assert (3 : Nat32 ** (4 : Nat32) == (81 : Nat32));
    assert (3 : Nat32 ** (7 : Nat32) == (2187 : Nat32));
    assert (3 : Nat32 ** (14 : Nat32) == (4782969 : Nat32));
    assert (3 : Nat32 ** (20 : Nat32) == (3486784401 : Nat32));
};

// Nat16 operations
do {
    func printN16ln(w : Nat16) {
      Prim.debugPrintNat(Prim.nat16ToNat w);
    };
    func printI16ln(w : Int16) {
      Prim.debugPrintInt(Prim.int16ToInt w);
    };

    let a : Nat16 = 4567;
    let b : Nat16 = 7;
    let c : Nat16 = 55734;
    let d : Int16 = -15;
    let e : Nat16 = 20000;


    printN16ln(c);
    printN16ln(^c);
    printN16ln(a +% c);
    printN16ln(c -% a);

// CHECK: call $checkpointDelta
    checkpointDelta();
// this is the value of a
// CHECK: i64.const {{1285496218637565952|1285566587381743616}}
// this is the value of b
// CHECK: i64.const {{1970324836974592|2040693581152256}}
// This is not a native Wasm i64 multiplication, we need to shift one of the args left by 16 bits!
// CHECK: i64.const 48
// CHECK-NEXT: i64.shr_u
// CHECK-NEXT: i64.mul
// CHECK-NEXT: i64.const 70368744177664
// CHECK-NEXT: i64.or
// CHECK-NEXT: call $printN16ln
    printN16ln(a *% b);
    printN16ln(a / b);
    printN16ln(c % a);
    printN16ln(a **% 2);

    printN16ln(a & c);
    printN16ln(a | c);
    printN16ln(a ^ c);
    printN16ln(a << b);

// CHECK: call $checkpointEcho
   checkpointEcho();
// this is the value of b
// CHECK: i64.const {{1970324836974592|2040693581152256}}
// This is not a native Wasm i32 left shift, we need to shift the second arg left by 16 bits and clamp it to 4 bits!
// CHECK: i64.const 48
// CHECK-NEXT: i64.shr_u
// CHECK-NEXT: i64.const 15
// CHECK-NEXT: i64.and
// CHECK-NEXT: i64.shr_u
// Then the result must be sanitised.
// CHECK-NEXT: i64.const -281474976710656
// CHECK-NEXT: i64.and
// CHECK-NEXT: i64.const 70368744177664
// CHECK-NEXT: i64.or
// CHECK-NEXT: call $printN16ln
    printN16ln(a >> b);
    printN16ln(Prim.int16ToNat16(d) >> 3); // -15 = 0xfff1 = 0b1111_1111_1111_0001 (shifted = 0b0001_1111_1111_1110 = 8190)
    printI16ln(d >> 3); // -15 = 0xfff1 = 0b1111_1111_1111_0001 (shifted = 0b1111_1111_1111_1110 = -2)

// CHECK: call $checkpointFoxtrot
   checkpointFoxtrot();
// this is the value of b
// CHECK: i64.const {{1970324836974592|2040693581152256}}
// CHECK: call $printN16ln
    printN16ln(c <<> b);

// CHECK: call $checkpointGolf
   checkpointGolf();
// this is the value of b
// CHECK: i64.const {{1970324836974592|2040693581152256}}
// CHECK: i64.rotr
    printN16ln(c <>> b);
    printI16ln(Prim.popcntInt16 d); // -15 = 0xfff1 = 0b1111_1111_1111_0001 (population = 13)
    printN16ln(Prim.clzNat16 e); // 20000 = 0x4e20 (leading zeros = 1)
    printN16ln(Prim.ctzNat16 e); // 20000 = 0x4e20 (trailing zeros = 5)
    printBit(Prim.btstNat16(e, 5 : Nat16)); // 20000 = 0x4e20 (result = true)
    printBit(Prim.btstNat16(e, 15 : Nat16)); // 20000 = 0x4e20 (result = false)
    printBit(Prim.btstNat16(e, 21 : Nat16)); // 20000 = 0x4e20 (mod 16, result = true)


    assert (3 : Nat16 ** (0 : Nat16) == (1 : Nat16));
    assert (3 : Nat16 ** (1 : Nat16) == (3 : Nat16));
    assert (3 : Nat16 ** (4 : Nat16) == (81 : Nat16));
    assert (3 : Nat16 ** (7 : Nat16) == (2187 : Nat16));
};

// Nat8 operations
do {
    func printN8ln(w : Nat8) {
      Prim.debugPrintNat(Prim.nat8ToNat w);
    };
    func printI8ln(w : Int8) {
      Prim.debugPrintInt(Prim.int8ToInt w);
    };

    let a : Nat8 = 67;
    let b : Nat8 = 7;
    let c : Nat8 = 34;
    let d : Int8 = -15;
    let e : Nat8 = 200;


    printN8ln(c);
    printN8ln(^c);
    printN8ln(a + c);
    printN8ln(c -% a);
// CHECK: call $checkpointHotel
    checkpointHotel();
// this is the value of b
// CHECK: i64.const {{504403158265495552|522417556774977536}}
// This is not a native Wasm i64 multiplication, we need to shift one of the args left by 56 bits!
// CHECK: i64.const 56
// CHECK-NEXT: i64.shr_u
// CHECK-NEXT: i64.mul
// CHECK: call $printN8ln
    printN8ln(a *% b);
    printN8ln(a / b);
    printN8ln(c % a);
    printN8ln(a **% 2);

    printN8ln(a & c);
    printN8ln(a | c);
    printN8ln(a ^ c);
    printN8ln(a << b);

// CHECK: call $checkpointIndia
    checkpointIndia();
// this is the value of b
// CHECK: i64.const {{504403158265495552|522417556774977536}}
// This is not a native Wasm i64 left shift, we need to shift the second arg left by 56 bits and clamp it to 3 bits!
// CHECK: i64.const 56
// CHECK-NEXT: i64.shr_u
// CHECK-NEXT: i64.const 7
// CHECK-NEXT: i64.and
// CHECK-NEXT: i64.shr_u
// Then the result must be sanitised.
// CHECK-NEXT: i64.const -72057594037927936
// CHECK-NEXT: i64.and
// CHECK-NEXT: i64.const 18014398509481984
// CHECK-NEXT: i64.or
// CHECK-NEXT: call $printN8ln
    printN8ln(a >> b);
    printN8ln(Prim.int8ToNat8(d) >> 3); // -15 = 0xf1 = 0b1111_0001 (shifted = 0b0001_1110 = 30)
    printI8ln(d >> 3); // -15 = 0xf1 = 0b1111_0001 (shifted = 0b1111_1110 = -2)

// CHECK: call $checkpointJuliett
    checkpointJuliett();
// this is the value of b
// CHECK: i64.const {{504403158265495552|522417556774977536}}
// CHECK: call $printN8ln
    printN8ln(c <<> b);
// this is the value of b
// CHECK: i64.const {{504403158265495552|522417556774977536}}
// CHECK: call $printN8ln
    printN8ln(c <>> b);
    printI8ln(Prim.popcntInt8 d); // -15 = 0xf1 = 0b1111_0001 (population = 5)
    printN8ln(Prim.clzNat8 e); // 200 = 0xC8 (leading zeros = 0)
    printN8ln(Prim.ctzNat8 e); // 200 = 0xC8 (trailing zeros = 3)
    printBit(Prim.btstNat8(e, 3 : Nat8)); // 200 = 0xC8 (result = true)
    printBit(Prim.btstNat8(e, 5 : Nat8)); // 200 = 0xC8 (result = false)
    printBit(Prim.btstNat8(e, 11 : Nat8)); // 200 = 0xC8 (mod 8, result = true)

    assert (3 : Nat8 ** (0 : Nat8) == (1 : Nat8));
    assert (3 : Nat8 ** (3 : Nat8) == (27 : Nat8));
    assert (3 : Nat8 ** (4 : Nat8) == (81 : Nat8));
    assert (3 : Nat8 ** (5 : Nat8) == (243 : Nat8));
};


// check whether patterns work

func w8 (n : Nat8) = assert (switch n { case 0 false; case 1 false; case 42 true; case _ false });
func w16 (n : Nat16) = assert (switch n { case 0 false; case 1 false; case 65000 true; case _ false });
func w32 (n : Nat32) = assert (switch n { case 0 false; case 1 false; case 4_294_967_295 true; case _ false });
func w64 (n : Nat64) = assert (switch n { case 0 false; case 1 false; case 42 true; case _ false });


w8 42;
w16 65000;
w32 4_294_967_295;
w64 42;
