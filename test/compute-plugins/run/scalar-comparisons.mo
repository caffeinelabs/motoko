//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";

// Multiple scalar specializations share a module and exercise tagged/boxed
// boundaries, signedness, UTF-8 ordering and equal values in different shapes.
let huge : Nat = 2 ** 300;
assert Prim.fixtureCompare<Nat>(0, huge) == -1;
assert Prim.fixtureCompare<Nat>(huge, 0) == 1;
assert Prim.fixtureCompare<Nat>(huge, huge + 0) == 0;
assert Prim.fixtureCompare<Int>(-huge, -1) == -1;
assert Prim.fixtureCompare<Int>(0, -huge) == 1;
assert Prim.fixtureCompare<Int>(-huge, -huge) == 0;
assert Prim.fixtureCompare<Nat8>(0, 255) == -1;
assert Prim.fixtureCompare<Int8>(-128, 127) == -1;
assert Prim.fixtureCompare<Int8>(127, -128) == 1;
assert Prim.fixtureCompare<Nat16>(65535, 0) == 1;
assert Prim.fixtureCompare<Int16>(-32768, 32767) == -1;
assert Prim.fixtureCompare<Nat32>(4294967295, 0) == 1;
assert Prim.fixtureCompare<Int32>(-2147483648, 2147483647) == -1;
assert Prim.fixtureCompare<Nat64>(18446744073709551615, 0) == 1;
assert Prim.fixtureCompare<Nat64>(0, 18446744073709551615) == -1;
assert Prim.fixtureCompare<Int64>(-9223372036854775808, 9223372036854775807) == -1;
assert Prim.fixtureCompare<Int64>(9223372036854775807, -9223372036854775808) == 1;
assert Prim.fixtureCompare<Nat64>(18446744073709551615, 18446744073709551615) == 0;
assert Prim.fixtureCompare<Int64>(-9223372036854775808, -9223372036854775808) == 0;
assert Prim.fixtureCompare<Bool>(false, true) == -1;
assert Prim.fixtureCompare<Bool>(Prim.fixtureTrue(), true) == 0;
assert Prim.fixtureCompare<Bool>(true, Prim.fixtureTrue()) == 0;
assert Prim.fixtureCompare<Char>('a', '☃') == -1;
assert Prim.fixtureCompare<Char>('💫', '☃') == 1;
assert Prim.fixtureCompare<Text>("a☃", "a💫") == -1;
var rope = "";
var i = 0;
while (i < 40) { rope #= "abcdefgh☃"; i += 1 };
assert Prim.fixtureCompare<Text>(rope # "x", rope # "y") == -1;
assert Prim.fixtureCompare<Text>(rope, "" # rope) == 0;
assert Prim.fixtureCompare<Blob>("\00\ff", "\00\ff\00") == -1;
assert Prim.fixtureCompare<Blob>("\ff", "\01") == 1;
let left = Prim.principalOfBlob("\00\ff");
let right = Prim.principalOfBlob("\00\ff\00");
assert Prim.fixtureCompare<Principal>(left, right) == -1;
