//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/callbacks
import Prim "mo:⛔";
// Exhaustively compare both eight-bit domains, including sign boundaries.
var low = 0;
while (low < 256) {
  var high = 0;
  while (high < 256) {
    let a = Prim.natToNat8(low); let b = Prim.natToNat8(high);
    assert Prim.fixtureCompare<Nat8>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
    let x = Prim.intToInt8(low - 128); let y = Prim.intToInt8(high - 128);
    assert Prim.fixtureCompare<Int8>(x, y) ==
      (if (x < y) -1 else if (x == y) 0 else 1);
    high += 1;
  };
  low += 1;
};

// Heap-loaded aliases retain the same canonical low tags as direct values.
type AliasNat16 = Nat16;
let valuesNat16 : [AliasNat16] = [0, 1, 32767, 32768, 65534, 65535];
for (a in valuesNat16.values()) {
  for (b in valuesNat16.values()) {
    assert Prim.fixtureCompare<AliasNat16>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
  };
};
type AliasInt16 = Int16;
let valuesInt16 : [AliasInt16] = [-32768, -32767, -1, 0, 1, 32767];
for (a in valuesInt16.values()) {
  for (b in valuesInt16.values()) {
    assert Prim.fixtureCompare<AliasInt16>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
  };
};
type AliasNat32 = Nat32;
let valuesNat32 : [AliasNat32] = [0, 1, 2147483647, 2147483648, 4294967294, 4294967295];
for (a in valuesNat32.values()) {
  for (b in valuesNat32.values()) {
    assert Prim.fixtureCompare<AliasNat32>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
  };
};
type AliasInt32 = Int32;
let valuesInt32 : [AliasInt32] = [-2147483648, -2147483647, -1, 0, 1, 2147483647];
for (a in valuesInt32.values()) {
  for (b in valuesInt32.values()) {
    assert Prim.fixtureCompare<AliasInt32>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
  };
};
type AliasChar = Char;
let valuesChar : [AliasChar] = ['\u{0}', 'a', '\u{d7ff}', '\u{e000}', '☃', '💫', '\u{ffff}', '\u{10000}', '\u{10fffe}', '\u{10ffff}'];
for (a in valuesChar.values()) {
  for (b in valuesChar.values()) {
    assert Prim.fixtureCompare<AliasChar>(a, b) ==
      (if (a < b) -1 else if (a == b) 0 else 1);
  };
};
