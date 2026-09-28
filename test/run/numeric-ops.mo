import Prim "mo:⛔";

// Testing utility

// expects 2 elements in results for each one in etalon
// asserts that those compare equal

func verify<T>(etalon : [T], results : [T], cmp : (T, T) -> Bool) {
  for i in etalon.keys()
  {
    assert cmp(etalon[i], results[i * 2]);
    assert cmp(etalon[i], results[i * 2 + 1]);
  }
};

func intCompare (a : Int, b : Int) : Bool = a == b;
func natCompare (a : Nat, b : Nat) : Bool = a == b;

// Numeric operators

func testNat(a : Nat, b : Nat) : [Nat] {
  let sum1 = a + b;
  let sum2 = (a + b) : Nat;
  let diff1 = a - b;
  let diff2 = (a - b) : Nat;
  let prod1 = a * b;
  let prod2 = (a * b) : Nat;
  let rat1 = a / b;
  let rat2 = (a / b) : Nat;
  let mod1 = a % b;
  let mod2 = (a % b) : Nat;
  let pow1 = a ** b;
  let pow2 = (a ** b) : Nat;
  [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
};

verify<Nat>([8, 2, 15, 1, 2, 125], testNat(5, 3), natCompare);
verify<Nat>([41, 21, 310, 3, 1, 819628286980801], testNat(31, 10), natCompare);
verify<Nat>([42, 20, 341, 2, 9, 25408476896404831], testNat(31, 11), natCompare);
verify<Nat>([1500000002, 1499999998, 3000000000, 750000000, 0, 2250000000000000000], testNat(1500000000, 2), natCompare);

func testInt(a : Int, b : Int) : [Int] {
  let pos1 = + a;
  let pos2 = (+ a) : Int;
  let neg1 = - a;
  let neg2 = (- a) : Int;
  let sum1 = a + b;
  let sum2 = (a + b) : Int;
  let diff1 = a - b;
  let diff2 = (a - b) : Int;
  let prod1 = a * b;
  let prod2 = (a * b) : Int;
  let rat1 = a / b;
  let rat2 = (a / b) : Int;
  let mod1 = a % b;
  let mod2 = (a % b) : Int;
  let pow1 = a ** b;
  let pow2 = (a ** b) : Int;
  [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
};

verify<Int>([3, -3, 8, -2, 15, 0, 3, 243], testInt(3, 5), intCompare);
verify<Int>([-31, 31, -21, -41, -310, -3, -1, 819628286980801], testInt(-31, 10), intCompare);
verify<Int>([-31, 31, -20, -42, -341, -2, -9, -25408476896404831], testInt(-31, 11), intCompare);
verify<Int>([-1500000000, 1500000000, -1499999998, -1500000002, -3000000000, -750000000, 0, 2250000000000000000], testInt(-1500000000, 2), intCompare);

func testIntAbs(a : Int) : [Int] {
  let abs1 = Prim.abs a;
  let abs2 = (Prim.abs a) : Int;
  let abs3 = Prim.abs (-a);
  let abs4 = (Prim.abs (-a)) : Int;
  [abs1, abs2, abs3, abs4]
};

verify<Int>([0, 0], testIntAbs(0), intCompare);
verify<Int>([1, 1], testIntAbs(1), intCompare);
verify<Int>([4567, 4567], testIntAbs(-4567), intCompare);
verify<Int>([0x40000000, 0x40000000], testIntAbs(-0x40000000), intCompare);
verify<Int>([0x40000001, 0x40000001], testIntAbs(-0x40000001), intCompare);
verify<Int>([0x80000000, 0x80000000], testIntAbs(-0x80000000), intCompare);

func testIntNegation(a : Int) : [Int] {
  let neg1 = -a;
  let neg2 = (-a) : Int;
  [neg1, neg2]
};

verify<Int>([0x80000000], testIntNegation(-0x80000000), intCompare);
verify<Int>([-0x80000000], testIntNegation(0x80000000), intCompare);
verify<Int>([0x40000000], testIntNegation(-0x40000000), intCompare);
verify<Int>([-0x40000000], testIntNegation(0x40000000), intCompare);
verify<Int>([0x30000000], testIntNegation(-0x30000000), intCompare);
verify<Int>([-0x30000000], testIntNegation(0x30000000), intCompare);
verify<Int>([3], testIntNegation(-3), intCompare);
verify<Int>([-3], testIntNegation(3), intCompare);

func testNatInt(a : Nat, b : Int) : [Int] {
  let pos1 = + a;
  let pos2 = (+ a) : Int;
  let neg1 = - a;
  let neg2 = (- a) : Int;
  let sum1 = a + b;
  let sum2 = (a + b) : Int;
  let sum3 = b + a;
  let sum4 = (b + a) : Int;
  let diff1 = a - b;
  let diff2 = (a - b) : Int;
  let diff3 = b - a;
  let diff4 = (b - a) : Int;
  let prod1 = a * b;
  let prod2 = (a * b) : Int;
  let prod3 = b * a;
  let prod4 = (b * a) : Int;
  let rat1 = a / b;
  let rat2 = (a / b) : Int;
  let rat3 = b / a;
  let rat4 = (b / a) : Int;
  let mod1 = a % b;
  let mod2 = (a % b) : Int;
  let mod3 = b % a;
  let mod4 = (b % a) : Int;
  let pow1 = a ** b;
  let pow2 = (a ** b) : Int;
  let pow3 = b ** a;
  let pow4 = (b ** a) : Int;
  [
    pos1,
    pos2,
    neg1,
    neg2,
    sum1,
    sum2,
    sum3,
    sum4,
    diff1,
    diff2,
    diff3,
    diff4,
    prod1,
    prod2,
    prod3,
    prod4,
    rat1,
    rat2,
    rat3,
    rat4,
    mod1,
    mod2,
    mod3,
    mod4,
    pow1,
    pow2,
    pow3,
    pow4
  ]
};

verify<Int>(
  [3, -3, 8, 8, -2, 2, 15, 15, 0, 1, 3, 2, 243, 125],
  testNatInt(3, 5),
  intCompare
);

func testFloat(a : Float, b : Float) : [Float] {
  let pos1 = + a;
  let pos2 = (+ a) : Float;
  let neg1 = - a;
  let neg2 = (- a) : Float;
  let sum1 = a + b;
  let sum2 = (a + b) : Float;
  let diff1 = a - b;
  let diff2 = (a - b) : Float;
  let prod1 = a * b;
  let prod2 = (a * b) : Float;
  let rat1 = a / b;
  let rat2 = (a / b) : Float;
  let pow1 = a ** b;
  let pow2 = (a ** b) : Float;
  [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, pow1, pow2]
};

verify<Float>([3, -3, 8, -2, 15, 0.6, 243.0], testFloat(3.0, 5.0), func (a : Float, b : Float) : Bool = a == b);

func testInt64(a : Int64, b : Int64) : [Int64] {
  let pos1 = + a;
  let pos2 = (+ a) : Int64;
  let neg1 = - a;
  let neg2 = (- a) : Int64;
  let sum1 = a + b;
  let sum2 = (a + b) : Int64;
  let diff1 = a - b;
  let diff2 = (a - b) : Int64;
  let prod1 = a * b;
  let prod2 = (a * b) : Int64;
  let rat1 = a / b;
  let rat2 = (a / b) : Int64;
  let mod1 = a % b;
  let mod2 = (a % b) : Int64;
  if b >= (0 : Int64)
  {
    let pow1 = a ** b;
    let pow2 = (a ** b) : Int64;
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
  }
  else
  {
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2]
  }
};

func int64Compare(a : Int64, b : Int64) : Bool = a == b;

verify<Int64>(
  [3, -3, 8, -2, 15, 0, 3, 243],
  testInt64(3, 5),
  int64Compare
);
verify<Int64>(
  [13, -13, 18, 8, 65, 2, 3, 371293],
  testInt64(13, 5),
  int64Compare
);
verify<Int64>(
  [-13, 13, -18, -8, 65, 2, -3],
  testInt64(-13, -5),
  int64Compare
);

func testInt32(a : Int32, b : Int32) : [Int32] {
  let pos1 = + a;
  let pos2 = (+ a) : Int32;
  let neg1 = - a;
  let neg2 = (- a) : Int32;
  let sum1 = a + b;
  let sum2 = (a + b) : Int32;
  let diff1 = a - b;
  let diff2 = (a - b) : Int32;
  let prod1 = a * b;
  let prod2 = (a * b) : Int32;
  let rat1 = a / b;
  let rat2 = (a / b) : Int32;
  let mod1 = a % b;
  let mod2 = (a % b) : Int32;
  if b >= (0 : Int32)
  {
    let pow1 = a ** b;
    let pow2 = (a ** b) : Int32;
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
  }
  else
  {
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2]
  }
};

func int32Compare(a : Int32, b : Int32) : Bool = a == b;

verify<Int32>(
  [3, -3, 8, -2, 15, 0, 3, 243],
  testInt32(3, 5),
  int32Compare
);
verify<Int32>(
  [13, -13, 18, 8, 65, 2, 3, 371293],
  testInt32(13, 5),
  int32Compare
);
verify<Int32>(
  [-13, 13, -18, -8, 65, 2, -3],
  testInt32(-13, -5),
  int32Compare
);

func testInt16(a : Int16, b : Int16) : [Int16] {
  let pos1 = + a;
  let pos2 = (+ a) : Int16;
  let neg1 = - a;
  let neg2 = (- a) : Int16;
  let sum1 = a + b;
  let sum2 = (a + b) : Int16;
  let diff1 = a - b;
  let diff2 = (a - b) : Int16;
  let prod1 = a * b;
  let prod2 = (a * b) : Int16;
  let rat1 = a / b;
  let rat2 = (a / b) : Int16;
  let mod1 = a % b;
  let mod2 = (a % b) : Int16;
  if b >= (0 : Int16) and a < (10 : Int16) {
    let pow1 = a ** b;
    let pow2 = (a ** b) : Int16;
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
  } else { [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2] }
};

func int16Compare(a : Int16, b : Int16) : Bool = a == b;

verify<Int16>(
  [3, -3, 8, -2, 15, 0, 3, 243],
  testInt16(3, 5),
  int16Compare
);
verify<Int16>(
  [13, -13, 18, 8, 65, 2, 3],
  testInt16(13, 5),
  int16Compare
);
verify<Int16>(
  [-13, 13, -18, -8, 65, 2, -3],
  testInt16(-13, -5),
  int16Compare
);

func testInt8(a : Int8, b : Int8) : [Int8] {
  let pos1 = + a;
  let pos2 = (+ a) : Int8;
  let neg1 = - a;
  let neg2 = (- a) : Int8;
  let sum1 = a + b;
  let sum2 = (a + b) : Int8;
  let diff1 = a - b;
  let diff2 = (a - b) : Int8;
  let prod1 = a * b;
  let prod2 = (a * b) : Int8;
  let rat1 = a / b;
  let rat2 = (a / b) : Int8;
  let mod1 = a % b;
  let mod2 = (a % b) : Int8;
  if b >= (0 : Int8) and b < (5 : Int8) {
    let pow1 = a ** b;
    let pow2 = (a ** b) : Int8;
    [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
  } else { [pos1, pos2, neg1, neg2, sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2] }
};

func int8Compare(a : Int8, b : Int8) : Bool = a == b;

verify<Int8>(
  [3, -3, 7, -1, 12, 0, 3, 81],
  testInt8(3, 4),
  int8Compare
);
verify<Int8>(
  [3, -3, 8, -2, 15, 0, 3],
  testInt8(3, 5),
  int8Compare
);
verify<Int8>(
  [13, -13, 18, 8, 65, 2, 3],
  testInt8(13, 5),
  int8Compare
);
verify<Int8>(
  [3, -3, -2, 8, -15, 0, 3],
  testInt8(3, -5),
  int8Compare
);
verify<Int8>(
  [13, -13, 8, 18, -65, -2, 3],
  testInt8(13, -5),
  int8Compare
);
verify<Int8>(
  [-13, 13, -18, -8, 65, 2, -3],
  testInt8(-13, -5),
  int8Compare
);

func testNat64(a : Nat64, b : Nat64) : [Nat64] {
  let sum1 = a + b;
  let sum2 = (a + b) : Nat64;
  let diff1 = a - b;
  let diff2 = (a - b) : Nat64;
  let prod1 = a * b;
  let prod2 = (a * b) : Nat64;
  let rat1 = a / b;
  let rat2 = (a / b) : Nat64;
  let mod1 = a % b;
  let mod2 = (a % b) : Nat64;
  let pow1 = a ** b;
  let pow2 = (a ** b) : Nat64;
  [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
};

func nat64Compare(a : Nat64, b : Nat64) : Bool = a == b;

verify<Nat64>(
  [8, 2, 15, 1, 2, 125],
  testNat64(5, 3),
  nat64Compare
);
verify<Nat64>(
  [18, 8, 65, 2, 3, 371293],
  testNat64(13, 5),
  nat64Compare
);

func testNat32(a : Nat32, b : Nat32) : [Nat32] {
  let sum1 = a + b;
  let sum2 = (a + b) : Nat32;
  let diff1 = a - b;
  let diff2 = (a - b) : Nat32;
  let prod1 = a * b;
  let prod2 = (a * b) : Nat32;
  let rat1 = a / b;
  let rat2 = (a / b) : Nat32;
  let mod1 = a % b;
  let mod2 = (a % b) : Nat32;
  /*let pow1 = a ** b;
  let pow2 = (a ** b) : Nat32;*/
  [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2 /*, pow1, pow2*/]
};

func nat32Compare(a : Nat32, b : Nat32) : Bool = a == b;

verify<Nat32>(
  [8, 2, 15, 1, 2 /*, 243*/],
  testNat32(5, 3),
  nat32Compare
);
verify<Nat32>(
  [18, 12, 45, 5, 0 /*, 243*/],
  testNat32(15, 3),
  nat32Compare
);

func testNat16(a : Nat16, b : Nat16) : [Nat16] {
  let sum1 = a + b;
  let sum2 = (a + b) : Nat16;
  let diff1 = a - b;
  let diff2 = (a - b) : Nat16;
  let prod1 = a * b;
  let prod2 = (a * b) : Nat16;
  let rat1 = a / b;
  let rat2 = (a / b) : Nat16;
  let mod1 = a % b;
  let mod2 = (a % b) : Nat16;
  /*let pow1 = a ** b;
  let pow2 = (a ** b) : Nat16;*/
  [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2 /*, pow1, pow2*/]
};

func nat16Compare(a : Nat16, b : Nat16) : Bool = a == b;

verify<Nat16>(
  [8, 2, 15, 1, 2 /*, 243*/],
  testNat16(5, 3),
  nat16Compare
);
verify<Nat16>(
  [18, 12, 45, 5, 0 /*, 243*/],
  testNat16(15, 3),
  nat16Compare
);

func testNat8(a : Nat8, b : Nat8) : [Nat8] {
  let sum1 = a + b;
  let sum2 = (a + b) : Nat8;
  let diff1 = a - b;
  let diff2 = (a - b) : Nat8;
  let prod1 = a * b;
  let prod2 = (a * b) : Nat8;
  let rat1 = a / b;
  let rat2 = (a / b) : Nat8;
  let mod1 = a % b;
  let mod2 = (a % b) : Nat8;
  if b >= (0 : Nat8) and a < (10 : Nat8) {
    let pow1 = a ** b;
    let pow2 = (a ** b) : Nat8;
    [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2, pow1, pow2]
  } else { [sum1, sum2, diff1, diff2, prod1, prod2, rat1, rat2, mod1, mod2] }
};

func nat8Compare(a : Nat8, b : Nat8) : Bool = a == b;

verify<Nat8>(
  [8, 2, 15, 1, 2, 125],
  testNat8(5, 3),
  nat8Compare
);
verify<Nat8>(
  [18, 12, 45, 5, 0],
  testNat8(15, 3),
  nat8Compare
);
