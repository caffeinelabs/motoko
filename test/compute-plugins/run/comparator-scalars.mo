//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/comparators
import Prim "mo:⛔";

type Order = { #less; #equal; #greater };
func check<T>(compare : (T, T) -> Order, values : [T]) {
  for (left in values.values()) {
    for (right in values.values()) {
      let expected : Int = switch (compare(left, right)) {
        case (#less) -1; case (#equal) 0; case (#greater) 1
      };
      assert Prim.fixtureAutoCompare<T>(compare, left, right) == expected;
      assert Prim.fixtureAutoComparePair<T>(compare, left, right).0 == expected;
      Prim.fixtureAutoCompareUnit<T>(compare, left, right);
      assert Prim.fixtureNestedCompare<T>(compare, left, right) == expected;
    }
  }
};

func nat(a : Nat, b : Nat) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func int(a : Int, b : Int) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func nat8(a : Nat8, b : Nat8) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func nat16(a : Nat16, b : Nat16) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func nat32(a : Nat32, b : Nat32) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func nat64(a : Nat64, b : Nat64) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func int8(a : Int8, b : Int8) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func int16(a : Int16, b : Int16) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func int32(a : Int32, b : Int32) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func int64(a : Int64, b : Int64) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func char(a : Char, b : Char) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func comparePrincipal(a : Principal, b : Principal) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func bool(a : Bool, b : Bool) : Order {
  if (a == b) #equal else if a #greater else #less
};
func text(a : Text, b : Text) : Order {
  let c = Prim.textCompare(a, b);
  if (c < 0) #less else if (c == 0) #equal else #greater
};
func blob(a : Blob, b : Blob) : Order {
  let c = Prim.blobCompare(a, b);
  if (c < 0) #less else if (c == 0) #equal else #greater
};

check<Nat>(nat, [0, 1, 9_223_372_036_854_775_808,
  340_282_366_920_938_463_463_374_607_431_768_211_456]);
check<Int>(int, [-340_282_366_920_938_463_463_374_607_431_768_211_456,
  -9_223_372_036_854_775_808, -1, 0, 1,
  340_282_366_920_938_463_463_374_607_431_768_211_456]);
check<Nat8>(nat8, [0, 127, 128, 255]);
check<Nat16>(nat16, [0, 32767, 32768, 65535]);
check<Nat32>(nat32, [0, 2147483647, 2147483648, 4294967295]);
check<Nat64>(nat64, [0, 1, 9_223_372_036_854_775_807,
  9_223_372_036_854_775_808, 18_446_744_073_709_551_615]);
check<Int8>(int8, [-128, -1, 0, 127]);
check<Int16>(int16, [-32768, -1, 0, 32767]);
check<Int32>(int32, [-2147483648, -1, 0, 2147483647]);
check<Int64>(int64, [-9_223_372_036_854_775_808, -1, 0,
  9_223_372_036_854_775_807]);
check<Char>(char, ['\00', 'a', '\u{10ffff}']);
check<Bool>(bool, [false, true]);
check<Text>(text, ["", "a", "aa", "ä", "😀"]);
check<Blob>(blob, ["", "\00", "\00\ff", "\ff"]);
check<Principal>(comparePrincipal, [Prim.principalOfBlob(""),
  Prim.principalOfBlob("\04"), Prim.principalOfBlob("\ff")]);

// The proof describes the comparator, rather than generic wrapper T.
check<Nat>(int, [0, 1, 340_282_366_920_938_463_463_374_607_431_768_211_456]);
