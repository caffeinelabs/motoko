import Prim "mo:⛔";

// Chains of bitwise operations, shifts and rotations on the small word types.
// Nat32 and Int32 are kept as plain i32 values in locals and across
// expressions; these tests exercise every consumer of such values (arithmetic,
// comparisons, stores, conversions, patterns, closures) with operands coming out
// of such chains, for all small word types.

func printN32(w : Nat32) { Prim.debugPrintNat(Prim.nat32ToNat(w)) };
func printI32(w : Int32) { Prim.debugPrintInt(Prim.int32ToInt(w)) };
func printN16(w : Nat16) { Prim.debugPrintNat(Prim.nat16ToNat(w)) };
func printI16(w : Int16) { Prim.debugPrintInt(Prim.int16ToInt(w)) };
func printN8(w : Nat8) { Prim.debugPrintNat(Prim.nat8ToNat(w)) };
func printI8(w : Int8) { Prim.debugPrintInt(Prim.int8ToInt(w)) };
func printBool(b : Bool) { Prim.debugPrint(debug_show(b)) };

// SHA-256 style mixing functions, as in the motivating issue
func bigSigma0(e : Nat32) : Nat32 { (e <>> 2) ^ (e <>> 13) ^ (e <>> 22) };

// The argument is unpacked into an i32 local once; the rotations and xors are
// single i32 instructions, with no masking, and the result is tagged once.
// CHECK: func $bigSigma1
// CHECK: i32.wrap_i64
// CHECK-NOT: i64.
// CHECK: i32.rotr
// CHECK-NOT: i64.
// CHECK: i32.rotr
// CHECK-NEXT: i32.xor
// CHECK-NOT: i64.
// CHECK: i32.rotr
// CHECK-NEXT: i32.xor
// CHECK-NEXT: i64.extend_i32_u
// CHECK-NEXT: i64.const 32
// CHECK-NEXT: i64.shl
// CHECK-NEXT: i64.const 1073741824
// CHECK-NEXT: i64.or
func bigSigma1(e : Nat32) : Nat32 { (e <>> 6) ^ (e <>> 11) ^ (e <>> 25) };
func smallSigma0(e : Nat32) : Nat32 { (e <>> 7) ^ (e <>> 18) ^ (e >> 3) };
func smallSigma1(e : Nat32) : Nat32 { (e <>> 17) ^ (e <>> 19) ^ (e >> 10) };

do {
  let e : Nat32 = 0x5bafecbd;
  let f : Nat32 = 0x1234abcd;

  // results passed to functions (Vanilla)
  printN32(bigSigma0(e));
  printN32(bigSigma1(e));
  printN32(smallSigma0(e));
  printN32(smallSigma1(e));
  assert bigSigma0(e) == 0x8cb3d33e;
  assert bigSigma1(e) == 0xb53394e2;
  assert smallSigma0(e) == 0x8aedf4a5;
  assert smallSigma1(e) == 0x0bdfed59;

  // not inside chains
  assert ((e & f) ^ ((^e) & 0x0f0f0f0f)) == 0x1624ab8f;
  assert ((e & f) ^ (e & 0x0f0f0f0f) ^ (f & 0x0f0f0f0f)) == 0x1b2faf8d;
  assert (^(e >> 4)) == 0xfa450134;
  assert ((^e) >> 4) == 0x0a450134;

  // arithmetic consumes chain results (needs clean operands)
  assert (bigSigma1(e) +% smallSigma0(f)) == 0x67c54d85;
  assert ((e >> 3) *% (f >> 5)) == 0xe44c7072;
  assert ((e >> 3) + (f >> 5)) == (192282007 + 9545054);
  assert ((e >> 1) / ((f >> 7) | 1)) == 0x142;
  assert ((e >> 1) % ((f >> 7) | 1)) == 0xb76f0;

  // left shifts and rotations consume chain results
  assert ((e <>> 3) << 2) == 0xadd7f65c;
  assert ((e >> 4) <<> 7) == 0xdd7f6582;
  assert ((e >> 4) <<> (f >> 28)) == ((e >> 4) <<> 1);
  assert ((e <>> 3) <>> (e >> 29)) == ((e <>> 3) <>> 2);

  // comparisons consume chain results
  printBool((e >> 5) < (f >> 5));
  printBool((e <>> 3) == (e <>> 35));
  printBool((e >> 5) <= (f >> 5));
  printBool((e >> 5) >= (f >> 5));
  printBool((e ^ f) != (e | f));
  assert not ((e >> 5) < (f >> 5));
  assert (e <>> 3) == (e <>> 35);

  // storing chain results
  var v : Nat32 = 0;
  v := e <>> 4;
  assert v == 0xd5bafecb;
  v ^= e >> 3;
  v |= e <<> 5;
  v &= ^(f >> 1);
  printN32(v);
  let arr : [var Nat32] = [var 0, 0, 0];
  arr[0] := e <>> 4;
  arr[1] := (arr[0] >> 1) ^ (arr[0] <>> 7);
  arr[2] := ^arr[1] & (e >> 2);
  printN32(arr[0]);
  printN32(arr[1]);
  printN32(arr[2]);
  assert arr[0] == 0xd5bafecb;
  let tuple = (e >> 1, e <>> 1);
  printN32(tuple.0);
  printN32(tuple.1);
  let opt = ?(e <>> 9);
  switch opt {
    case (?w) { printN32(w) };
    case null { assert false };
  };

  // branches: unsanitized vs sanitized joins
  let w1 = if (e > f) { e <>> 3 } else { e };
  let w2 = if (e < f) { e <>> 3 } else { f };
  let w3 = if (e > f) { e >> 3 } else { e <>> 3 };
  printN32(w1);
  printN32(w2);
  printN32(w3);
  assert w1 == 0xab75fd97;
  assert w2 == f;
  assert w3 == (e >> 3);
  let w4 = switch (e & 3) {
    case 1 { e <>> 1 };
    case 2 { e >> 2 };
    case _ { e };
  };
  printN32(w4);
  assert w4 == (e <>> 1);

  // conversions and primitives consume chain results
  assert Prim.nat32ToNat(e >> 3) == 192282007;
  assert Prim.nat32ToNat(e <>> 4) == 3585801931;
  assert Prim.nat32ToNat16((e <>> 4) >> 16) == 0xd5ba;
  assert Prim.nat32ToNat64(e <>> 4) == 0xd5bafecb;
  assert Prim.nat32ToInt32(e <>> 4) == -709165365;
  assert Prim.popcntNat32(e >> 3) == 20;
  assert Prim.clzNat32(e >> 7) == 8;
  assert Prim.ctzNat32(e <>> 4) == 0;
  assert Prim.btstNat32(e <>> 4, e >> 30);
  assert Prim.btstNat32(e <>> 4, 1);
  assert not Prim.btstNat32(e <>> 4, 2);
  assert Prim.nat32ToChar((e >> 20) ^ (f >> 24)) == '\u{5a8}';

  // patterns match chain results
  switch (e >> 28) {
    case 5 {};
    case _ { assert false };
  };
  switch (e <>> 4) {
    case 0xd5bafecb {};
    case _ { assert false };
  };
};

// Int32: arithmetic shifts and rotations
do {
  let d : Int32 = -1216614433;
  printI32(d >> 4);
  printI32(d <>> 5);
  printI32((d >> 4) ^ (d <>> 5) ^ (d <<> 9));
  printI32(^(d >> 7));
  assert (d >> 4) == -76038403;
  assert (d <>> 5) == -38019202;
  assert ((d >> 4) ^ (d <>> 5) ^ (d <<> 9)) == -250355987;
  assert (^(d >> 7)) == 9504800;
  assert ((d >> 3) ^ 0x7fffffff) == -1995406844;
  assert (d >> 31) == -1;
  assert (d >> 35) == (d >> 3);
  assert ((d >> 3) < 0);
  assert ((d >> 3) < (d >> 4));
  assert ((d >> 3) + (d >> 4)) == (-152076805 - 76038403);
  assert ((d >> 3) -% (d <>> 7)) == (-152076805 -% (d <>> 7));
  assert ((d >> 3) * 2) == -304153610;
  assert ((d >> 3) / 7) == -21725257;
  assert ((d >> 3) % 7) == -6;
  assert (-(d >> 3)) == 152076805;
  assert Prim.int32ToInt(d >> 3) == -152076805;
  assert Prim.int32ToInt64(d >> 3) == -152076805;
  assert Prim.int32ToNat32(d >> 3) == 4142890491;
  assert Prim.int32ToInt16(d >> 20) == -1161;
  assert Prim.popcntInt32(d >> 3) == 26;
  var i : Int32 = 1;
  i := d >> 1;
  i ^= d <>> 3;
  i &= ^(d >> 2);
  printI32(i);
  let j = if (d < 0) { d >> 1 } else { d };
  assert j == (d >> 1);
  switch (d >> 28) {
    case (-5) {};
    case _ { assert false };
  };
};

// Nat16 and Int16
do {
  let h : Nat16 = 0x5bbd;
  let g : Nat16 = 0xa1c3;
  printN16((h <>> 4) ^ (h <>> 9) ^ (h >> 3));
  printN16((g <<> 3) | (h >> 2));
  printN16((^(h >> 2)) ^ (g <>> 5));
  assert ((h <>> 4) ^ (h <>> 9) ^ (h >> 3)) == 0x61;
  assert ((g <<> 3) | (h >> 2)) == 0x1eff;
  assert ((^(h >> 2)) ^ (g <>> 5)) == 0xf41e;
  assert ((h <>> 4) +% (g >> 3)) == 0xe9f3;
  assert ((h <>> 4) + (g >> 3)) == 0xe9f3;
  assert ((h >> 3) *% (g >> 9)) == 38192;
  assert ((h <>> 4) << 4) == 0x5bb0;
  assert ((h >> 4) <<> 4) == 0x5bb0;
  assert ((h >> 2) < (g >> 2));
  assert ((h <>> 4) == (h <>> 20));
  assert Prim.nat16ToNat(h <>> 4) == 0xd5bb;
  assert Prim.nat16ToNat8((h <>> 4) >> 8) == 0xd5;
  assert Prim.nat16ToNat32(h <>> 4) == 0xd5bb;
  assert Prim.popcntNat16(h >> 3) == 9;
  var m : Nat16 = 0;
  m := h <>> 4;
  m ^= g >> 1;
  printN16(m);
  switch (h <>> 4) {
    case 0xd5bb {};
    case _ { assert false };
  };

  let d : Int16 = -15;
  printI16(d >> 3);
  printI16(d <>> 3);
  printI16((d >> 3) ^ (d <<> 2) ^ ((^d) >> 4));
  assert (d >> 3) == -2;
  assert (d <>> 3) == 16382;
  assert ((d >> 3) ^ (d <<> 2) ^ ((^d) >> 4)) == 57;
  assert ((d >> 3) + (d <>> 3)) == 16380;
  assert ((d >> 3) < 0);
  assert Prim.int16ToInt(d >> 3) == -2;
  assert Prim.int16ToInt32(d >> 3) == -2;
  assert Prim.int16ToNat16(d >> 3) == 0xfffe;
};

// Nat8 and Int8
do {
  let a : Nat8 = 0x56;
  let b : Nat8 = 0xc3;
  printN8((a <>> 3) ^ (a <>> 5) ^ (a >> 2));
  printN8(((b <<> 1) & (a >> 1)) | (b <>> 2));
  printN8((^(a >> 3)) ^ (b <<> 6));
  assert ((a <>> 3) ^ (a <>> 5) ^ (a >> 2)) == 0x6d;
  assert (((b <<> 1) & (a >> 1)) | (b <>> 2)) == 0xf3;
  assert ((^(a >> 3)) ^ (b <<> 6)) == 0x5;
  assert ((a >> 1) + (b >> 1)) == 0x8c;
  assert ((a <>> 3) +% (b <>> 3)) == 0x42;
  assert ((a >> 2) *% (b >> 5)) == 0x7e;
  assert ((a >> 2) << 3) == 0xa8;
  assert ((a >> 3) <<> 3) == 0x50;
  assert ((a >> 1) < (b >> 1));
  assert ((a <>> 3) == (a <>> 11));
  assert Prim.nat8ToNat(a <>> 3) == 0xca;
  assert Prim.nat8ToNat16(a <>> 3) == 0xca;
  assert Prim.nat16ToNat32(Prim.nat8ToNat16(a <>> 3)) == 0xca;
  assert Prim.popcntNat8(a >> 1) == 4;
  var n : Nat8 = 0;
  n := a <>> 3;
  n ^= b >> 1;
  n |= a <<> 1;
  printN8(n);
  switch (a <>> 3) {
    case 0xca {};
    case _ { assert false };
  };
  let blob = Prim.arrayToBlob([a <>> 3, (a >> 1) ^ b, ^(b >> 2)]);
  Prim.debugPrint(debug_show(blob));

  let d : Int8 = -15;
  printI8(d >> 3);
  printI8(d <>> 3);
  printI8((d >> 3) ^ (d <<> 2) ^ ((^d) >> 4));
  assert (d >> 3) == -2;
  assert (d <>> 3) == 62;
  assert ((d >> 3) ^ (d <<> 2) ^ ((^d) >> 4)) == 57;
  assert ((d >> 3) + (d <>> 3)) == 60;
  assert ((d >> 3) < 0);
  assert Prim.int8ToInt(d >> 3) == -2;
  assert Prim.int8ToInt16(d >> 3) == -2;
  assert Prim.int8ToNat8(d >> 3) == 0xfe;
};

// Unsanitized results captured by closures and crossing function boundaries
do {
  let e : Nat32 = 0x5bafecbd;
  let r = e <>> 6;
  func get() : Nat32 { r ^ (e >> 3) };
  printN32(get());
  let mix = func(x : Nat32) : Nat32 { (x <>> 7) ^ (x >> 3) };
  printN32(mix(e <>> 6));
  printN32(mix(mix(e)));
  assert get() == ((e <>> 6) ^ (e >> 3));
  assert mix(mix(e)) == ((mix(e) <>> 7) ^ (mix(e) >> 3));
};
