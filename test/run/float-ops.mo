//MOC-FLAG -A M0266
// This test exercises float ops with deliberately over-precise literals
// (a 19-digit pi, ~10^101 magnitudes), so silence the excess-precision warning.
import Prim "mo:⛔";

func isNegative(number: Float): Bool {
    Prim.floatCopySign(1.0, number) < 0.0
};

func negate(number: Float): Float {
    Prim.floatCopySign(number, if (isNegative(number)) { 1.0 } else { -1.0 })
};

let positiveZero = 0.0;
let negativeZero = -0.0;
assert(negate(positiveZero) == negativeZero);
assert(negate(negativeZero) == positiveZero);
assert (not isNegative(positiveZero));
assert (isNegative(negativeZero));
assert (isNegative(negate(positiveZero)));
assert (not isNegative(negate(negativeZero)));
Prim.debugPrint(debug_show(positiveZero));
Prim.debugPrint(debug_show(negativeZero));

let positiveNaN = Prim.floatCopySign(0.0/0.0, 1.0); // issue https://github.com/dfinity/motoko/issues/3647
let negativeNaN = -positiveNaN;
assert(positiveNaN != positiveNaN);
assert(positiveNaN != negativeNaN);
assert(negativeNaN != positiveNaN);
assert(negativeNaN != negativeNaN);
assert (not isNegative(positiveNaN));
assert (isNegative(negativeNaN));
assert (isNegative(negate(positiveNaN)));
assert (not isNegative(negate(negativeNaN)));
// Difference between 32-bit and 64-bit version. 64-bit is even more accurate (with correct sign bit on `nan`).
// Prim.debugPrint(debug_show(positiveNaN));
// Prim.debugPrint(debug_show(negativeNaN));

assert (Prim.floatAbs(9.7) == 9.7);
assert (Prim.floatAbs(-9.7) == 9.7);

assert (Prim.floatSqrt(0.0) == 0.0);
assert (Prim.floatSqrt(25.0) == 5.0);

assert (Prim.floatCeil(9.7) == 10.0);
assert (Prim.floatCeil(-9.7) == -9.0);

assert (Prim.floatFloor(9.7) == 9.0);
assert (Prim.floatFloor(-9.7) == -10.0);

assert (Prim.floatTrunc(9.7) == 9.0);
assert (Prim.floatTrunc(-9.7) == -9.0);

assert (Prim.floatNearest(9.7) == 10.0);
assert (Prim.floatNearest(-9.7) == -10.0);
assert (Prim.floatNearest(9.4) == 9.0);
assert (Prim.floatNearest(-9.4) == -9.0);

assert (Prim.floatMin(9.7, 4.2) == 4.2);
assert (Prim.floatMin(-9.7, 4.2) == -9.7);

assert (Prim.floatMax(9.7, 4.2) == 9.7);
assert (Prim.floatMax(-9.7, 4.2) == 4.2);

assert (Prim.floatCopySign(-9.7, 4.2) == 9.7);
assert (Prim.floatCopySign(9.7, -4.2) == -9.7);

assert (-9.7 < 4.2);
assert (-9.7 <= -9.7);
assert (-9.7 <= 4.2);

assert (-9.7 >= -9.7);
assert (9.7 >= -4.2);
assert (9.7 > -4.2);

assert (-9.7 != 4.2);

Prim.debugPrint(debug_show(+5.1 % +3.0));
Prim.debugPrint(debug_show(-5.1 % +3.0));
Prim.debugPrint(debug_show(+5.1 % -3.0));
Prim.debugPrint(debug_show(-5.1 % -3.0));

// Trigonometric

let pi = 3.141592653589793238;
let ninetyDegrees = (pi / 2.0);
assert (Prim.sin(0.0) == 0.0);
assert (Prim.sin(ninetyDegrees) == 1.0);
assert (Prim.cos(0.0) == 1.0);
assert (Prim.cos(pi) == -1.0);

let fortyFiveDegrees = (pi / 4.0);
assert (Prim.sin(fortyFiveDegrees) / Prim.cos(fortyFiveDegrees) == Prim.tan(fortyFiveDegrees));

let someRandomAngle = 0.42;
assert Prim.arcsin(Prim.sin(someRandomAngle)) == someRandomAngle;
assert Prim.floatAbs(Prim.arccos(Prim.cos(someRandomAngle)) - someRandomAngle) < 0.00000000000000006;
assert Prim.arctan(Prim.tan(someRandomAngle)) == someRandomAngle;

let sqrt2over2 : Float = Prim.floatSqrt(2) / 2;
assert Prim.floatAbs(Prim.arcsin(sqrt2over2) - fortyFiveDegrees) < 0.0000000000000002;
assert Prim.arcsin(1.0) == ninetyDegrees;

assert Prim.arccos(sqrt2over2) == fortyFiveDegrees;
assert Prim.arccos(1.0) == 0.0;

assert Prim.arctan(1.0) == fortyFiveDegrees;

assert Prim.arctan2(sqrt2over2, sqrt2over2) == fortyFiveDegrees;
assert Prim.arctan2(-sqrt2over2, sqrt2over2) == -fortyFiveDegrees;
assert Prim.arctan2(1, 0) == ninetyDegrees;
assert Prim.arctan2(0, 0) == 0.0;
assert Prim.arctan2(-1, 0) == -ninetyDegrees;

// Transcendental

assert Prim.exp(0.0) == 1.0;

let someRandomReal = 7.4225;
assert Prim.log(Prim.exp(someRandomReal)) == someRandomReal;
assert Prim.floatAbs(Prim.exp(Prim.log(someRandomReal)) - someRandomReal) < 0.000000000000001;

// Conversions
assert (Prim.floatToInt64(pi) == (3 : Int64));
assert (Prim.floatToInt64(-pi) == (-3 : Int64));

assert (Prim.int64ToFloat(42) == 42.0);
assert (Prim.int64ToFloat(-42) == -42.0);

assert (Prim.floatToInt(pi) == 3);
assert (Prim.floatToInt(-pi) == -3);
assert (Prim.floatToInt(pi * 40_000_000_000) == 125_663_706_143);
assert (Prim.floatToInt(-pi * 40_000_000_000) == -125_663_706_143);
assert (Prim.floatToInt(1073741824.1) == 1073741824); // for gauging the tiny bignum optimisation
assert (Prim.floatToInt(1073741823.8) == 1073741823);
assert (Prim.floatToInt(1073741823.5) == 1073741823);
assert (Prim.floatToInt(1073741823.1) == 1073741823);
assert (Prim.floatToInt(1073741822.9) == 1073741822);
assert (Prim.floatToInt(-1073741823.9) == -1073741823);
assert (Prim.floatToInt(-1073741824.1) == -1073741824);
assert (Prim.floatToInt(-1073741824.5) == -1073741824);
assert (Prim.floatToInt(-1073741824.8) == -1073741824);
assert (Prim.floatToInt(-1073741825.1) == -1073741825);

assert (Prim.floatToInt(1.7e308) == 169_999_999_999_999_993_883_079_578_865_998_174_333_346_074_304_075_874_502_773_119_193_537_729_178_160_565_864_330_091_787_584_707_988_572_262_467_983_188_919_169_916_105_593_357_174_268_369_962_062_473_635_296_474_636_515_660_464_935_663_040_684_957_844_303_524_367_815_028_553_272_712_298_986_386_310_828_644_513_212_353_921_123_253_311_675_499_856_875_650_512_437_415_429_217_994_623_324_794_855_339_589_632);
assert (Prim.floatToInt(-1.7e308) == -169_999_999_999_999_993_883_079_578_865_998_174_333_346_074_304_075_874_502_773_119_193_537_729_178_160_565_864_330_091_787_584_707_988_572_262_467_983_188_919_169_916_105_593_357_174_268_369_962_062_473_635_296_474_636_515_660_464_935_663_040_684_957_844_303_524_367_815_028_553_272_712_298_986_386_310_828_644_513_212_353_921_123_253_311_675_499_856_875_650_512_437_415_429_217_994_623_324_794_855_339_589_632);

assert (Prim.intToFloat(42) == 42.0);
assert (Prim.intToFloat(-42) == -42.0);
assert (Prim.intToFloat(42_000_000_000) == 42000000000.0);
assert (Prim.intToFloat(-42_000_000_000) == -42000000000.0);

Prim.debugPrint(debug_show(Prim.intToFloat(3 ** 7777))); // inf
assert (Prim.intToFloat(17 * 10 ** 100) == 170000000000000008531498165336373280720519466057626503553503085638131822882333707250541845375191351296.000000);

Prim.debugPrint(debug_show(Prim.intToFloat(-3 ** 7777))); // -inf
assert (Prim.intToFloat(-17 * 10 ** 100) == -170000000000000008531498165336373280720519466057626503553503085638131822882333707250541845375191351296.000000);

Prim.debugPrint(debug_show(Prim.intToFloat(17 * 10 ** 307)));
Prim.debugPrint(debug_show(Prim.intToFloat(-17 * 10 ** 307)));
