import Prim "mo:⛔";

// Nat <--> Nat8

func test_Nat_Nat8(n1 : Nat, n2 : Nat8) {
  assert Prim.natToNat8 n1 == n2;
  assert Prim.intToNat8Wrap n1 == n2;
  assert n1 == Prim.nat8ToNat n2;
};
func wrap_Nat_Nat8(n1 : Nat, n2 : Nat8) {
  assert Prim.natToNat8(n1 % 2 ** 8) == n2;
  assert Prim.intToNat8Wrap n1 == n2;
  assert n1 % 2 ** 8 == Prim.nat8ToNat n2;
};

test_Nat_Nat8(0, 0);
test_Nat_Nat8(1, 1);
test_Nat_Nat8(0xff, 0xff);
wrap_Nat_Nat8(0x100, 0);
wrap_Nat_Nat8(0x101, 1);

// Nat <--> Nat16

func test_Nat_Nat16(n1 : Nat, n2 : Nat16) {
  assert Prim.natToNat16 n1 == n2;
  assert Prim.intToNat16Wrap n1 == n2;
  assert n1 == Prim.nat16ToNat n2;
};
func wrap_Nat_Nat16(n1 : Nat, n2 : Nat16) {
  assert Prim.natToNat16(n1 % 2 ** 16) == n2;
  assert Prim.intToNat16Wrap n1 == n2;
  assert n1 % 2 ** 16 == Prim.nat16ToNat n2;
};

test_Nat_Nat16(0, 0);
test_Nat_Nat16(1, 1);
test_Nat_Nat16(0xffff, 0xffff);
wrap_Nat_Nat16(0x10000, 0);
wrap_Nat_Nat16(0x10001, 1);

// Nat <--> Nat32

func test_Nat_Nat32(n1 : Nat, n2 : Nat32) {
  assert Prim.natToNat32 n1 == n2;
  assert Prim.intToNat32Wrap n1 == n2;
  assert n1 == Prim.nat32ToNat n2;
};
func wrap_Nat_Nat32(n1 : Nat, n2 : Nat32) {
  assert Prim.natToNat32(n1 % 2 ** 32) == n2;
  assert Prim.intToNat32Wrap n1 == n2;
  assert n1 % 2 ** 32 == Prim.nat32ToNat n2;
};

test_Nat_Nat32(0, 0);
test_Nat_Nat32(1, 1);
test_Nat_Nat32(0xffffffff, 0xffffffff);
wrap_Nat_Nat32(0x100000000, 0);
wrap_Nat_Nat32(0x100000001, 1);

// Nat <--> Nat64

func test_Nat_Nat64(n1 : Nat, n2 : Nat64) {
  assert Prim.natToNat64 n1 == n2;
  assert Prim.intToNat64Wrap n1 == n2;
  assert n1 == Prim.nat64ToNat n2;
};
func wrap_Nat_Nat64(n1 : Nat, n2 : Nat64) {
  assert Prim.natToNat64(n1 % 2 ** 64) == n2;
  assert Prim.intToNat64Wrap n1 == n2;
  assert n1 % 2 ** 64 == Prim.nat64ToNat n2;
};

test_Nat_Nat64(0, 0);
test_Nat_Nat64(1, 1);
test_Nat_Nat64(0xffffffffffffffff, 0xffffffffffffffff);
wrap_Nat_Nat64(0x10000000000000000, 0);
wrap_Nat_Nat64(0x10000000000000001, 1);

// Int <--> Int8

func test_Int_Int8(n1 : Int, n2 : Int8) {
  assert Prim.intToInt8 n1 == n2;
  assert Prim.intToInt8Wrap n1 == n2;
  assert n1 == Prim.int8ToInt n2;
};
func wrap_Int_Int8(n1 : Int, n2 : Int8) {
  var n1_wrapped = n1 % 2 ** 8;
  if n1_wrapped >= 2 ** 7 { n1_wrapped -= 2 ** 8 };
  assert Prim.intToInt8 n1_wrapped == n2;
  assert Prim.intToInt8Wrap n1 == n2;
  assert n1_wrapped == Prim.int8ToInt n2;
};

test_Int_Int8(0, 0);
test_Int_Int8(1, 1);
test_Int_Int8(-1, -1);
test_Int_Int8(0x7f, 0x7f);
test_Int_Int8(-0x7f, -0x7f);
test_Int_Int8(-0x80, -0x80);
wrap_Int_Int8(0x80, -0x80);
wrap_Int_Int8(0x81, -0x7f);

// Int <--> Int16

func test_Int_Int16(n1 : Int, n2 : Int16) {
  assert Prim.intToInt16 n1 == n2;
  assert Prim.intToInt16Wrap n1 == n2;
  assert n1 == Prim.int16ToInt n2;
};
func wrap_Int_Int16(n1 : Int, n2 : Int16) {
  var n1_wrapped = n1 % 2 ** 16;
  if n1_wrapped >= 2 ** 15 { n1_wrapped -= 2 ** 16 };
  assert Prim.intToInt16 n1_wrapped == n2;
  assert Prim.intToInt16Wrap n1 == n2;
  assert n1_wrapped == Prim.int16ToInt n2;
};

test_Int_Int16(0, 0);
test_Int_Int16(1, 1);
test_Int_Int16(-1, -1);
test_Int_Int16(0x7fff, 0x7fff);
test_Int_Int16(-0x7fff, -0x7fff);
test_Int_Int16(-0x8000, -0x8000);
wrap_Int_Int16(0x8000, -0x8000);
wrap_Int_Int16(0x8001, -0x7fff);

// Int <--> Int32

func test_Int_Int32(n1 : Int, n2 : Int32) {
  assert Prim.intToInt32 n1 == n2;
  assert Prim.intToInt32Wrap n1 == n2;
  assert n1 == Prim.int32ToInt n2;
};
func wrap_Int_Int32(n1 : Int, n2 : Int32) {
  var n1_wrapped = n1 % 2 ** 32;
  if n1_wrapped >= 2 ** 31 { n1_wrapped -= 2 ** 32 };
  assert Prim.intToInt32 n1_wrapped == n2;
  assert Prim.intToInt32Wrap n1 == n2;
  assert n1_wrapped == Prim.int32ToInt n2;
};

test_Int_Int32(0, 0);
test_Int_Int32(1, 1);
test_Int_Int32(-1, -1);
test_Int_Int32(0x7fffffff, 0x7fffffff);
test_Int_Int32(-0x7fffffff, -0x7fffffff);
test_Int_Int32(-0x80000000, -0x80000000);
wrap_Int_Int32(0x80000000, -0x80000000);
wrap_Int_Int32(0x80000001, -0x7fffffff);

// Int <--> Int64

func test_Int_Int64(n1 : Int, n2 : Int64) {
  assert Prim.intToInt64 n1 == n2;
  assert Prim.intToInt64Wrap n1 == n2;
  assert n1 == Prim.int64ToInt n2;
};
func wrap_Int_Int64(n1 : Int, n2 : Int64) {
  var n1_wrapped = n1 % 2 ** 64;
  if n1_wrapped >= 2 ** 63 { n1_wrapped -= 2 ** 64 };
  assert Prim.intToInt64 n1_wrapped == n2;
  assert Prim.intToInt64Wrap n1 == n2;
  assert n1_wrapped == Prim.int64ToInt n2;
};

test_Int_Int64(0, 0);
test_Int_Int64(1, 1);
test_Int_Int64(-1, -1);
test_Int_Int64(0x7fffffffffffffff, 0x7fffffffffffffff);
test_Int_Int64(-0x7fffffffffffffff, -0x7fffffffffffffff);
test_Int_Int64(-0x8000000000000000, -0x8000000000000000);
wrap_Int_Int64(0x8000000000000000, -0x8000000000000000);
wrap_Int_Int64(0x8000000000000001, -0x7fffffffffffffff);

// NatN <--> Nat2N
func test_Nat8_Nat16(n1 : Nat8, n2 : Nat16) {
  assert Prim.nat8ToNat16 n1 == n2;
  assert Prim.nat16ToNat8 n2 == n1;
};
func test_Nat16_Nat32(n1 : Nat16, n2 : Nat32) {
  assert Prim.nat16ToNat32 n1 == n2;
  assert Prim.nat32ToNat16 n2 == n1;
};
func test_Nat32_Nat64(n1 : Nat32, n2 : Nat64) {
  assert Prim.nat32ToNat64 n1 == n2;
  assert Prim.nat64ToNat32 n2 == n1;
};

test_Nat8_Nat16(0, 0);
test_Nat8_Nat16(1, 1);
test_Nat8_Nat16(0xff, 0xff);
test_Nat16_Nat32(0, 0);
test_Nat16_Nat32(1, 1);
test_Nat16_Nat32(0xffff, 0xffff);
test_Nat32_Nat64(0, 0);
test_Nat32_Nat64(1, 1);
test_Nat32_Nat64(0xffffffff, 0xffffffff);

// IntN <--> Int2N
func test_Int8_Int16(n1 : Int8, n2 : Int16) {
  assert Prim.int8ToInt16 n1 == n2;
  assert Prim.int16ToInt8 n2 == n1;
};

func test_Int16_Int32(n1 : Int16, n2 : Int32) {
  assert Prim.int16ToInt32 n1 == n2;
  // "side effect" should only be printed once
  assert Prim.int32ToInt16((Prim.debugPrint "side effect", n2).1) == n1;
};

func test_Int32_Int64(n1 : Int32, n2 : Int64) {
  assert Prim.int32ToInt64 n1 == n2;
  assert Prim.int64ToInt32 n2 == n1;
};

test_Int8_Int16(0, 0);
test_Int8_Int16(1, 1);
test_Int8_Int16(-1, -1);
test_Int8_Int16(0x7f, 0x7f);
test_Int8_Int16(-0x7f, -0x7f);
test_Int8_Int16(-0x80, -0x80);
test_Int16_Int32(0, 0);
test_Int16_Int32(1, 1);
test_Int16_Int32(-1, -1);
test_Int16_Int32(0x7fff, 0x7fff);
test_Int16_Int32(-0x7fff, -0x7fff);
test_Int16_Int32(-0x8000, -0x8000);
test_Int32_Int64(0, 0);
test_Int32_Int64(1, 1);
test_Int32_Int64(-1, -1);
test_Int32_Int64(0x7fffffff, 0x7fffffff);
test_Int32_Int64(-0x7fffffff, -0x7fffffff);
test_Int32_Int64(-0x80000000, -0x80000000);
