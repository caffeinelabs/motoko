/**
[#mod-Hash]
= `Hash` -- Hash values
*/

import Prim "mo:⛔";
import Iter "iter";

module {
  /**
  Hash values represent a string of _hash bits_, packed into a `Nat32`.
  */
  public type Hash = Nat32;

  /**
  The hash length, always 31.
  */
  public let length : Nat = 31; // Why not 32?

  public let hashOfInt : Int -> Hash = func(i) {
    let j = Prim.int32ToNat32(Prim.intToInt32(i));
    hashNat8s(
      [
        j & (255 << 0),
        j & (255 << 8),
        j & (255 << 16),
        j & (255 << 24)
      ]
    );
  };

  /**
  WARNING: This only hashes the lowest 32 bits of the `Int`
  */
  public let hashOfIntAcc : (Hash, Int) -> Hash = func(h1, i) {
    let j = Prim.int32ToNat32(Prim.intToInt32(i));
    hashNat8s(
      [
        h1,
        j & (255 << 0),
        j & (255 << 8),
        j & (255 << 16),
        j & (255 << 24)
      ]
    );
  };

  /**
  WARNING: This only hashes the lowest 32 bits of the `Int`
  */
  public let hashOfText : Text -> Hash = func(t) {
    var x = 0 : Nat32;
    for c in t.chars() {
      x := x ^ Prim.charToNat32(c);
    };
    return x
  };

  /**
  Project a given bit from the bit vector.
  */
  public let getHashBit : (Hash, Nat) -> Bool = func(h, pos) {
    assert (pos <= length);
    (h & (Prim.natToNat32(1) << Prim.natToNat32(pos))) != Prim.natToNat32(0)
  };

  /**
  Test if two hashes are equal
  */
  public let hashEq : (Hash, Hash) -> Bool = func(ha, hb) {
    ha == hb
  };

  public let bitsPrintRev : Hash -> () = func(bits) {
    for j in Iter.range(0, length - 1) {
      if getHashBit(bits, j) {
        Prim.debugPrint "1"
      } else {
        Prim.debugPrint "0"
      }
    }
  };

  public let hashPrintRev : Hash -> () = func(bits) {
    for j in Iter.range(length - 1, 0) {
      if getHashBit(bits, j) {
        Prim.debugPrint "1"
      } else {
        Prim.debugPrint "0"
      }
    }
  };

  /**
  Jenkin's one at a time:
  https://en.wikipedia.org/wiki/Jenkins_hash_function#one_at_a_time

  The input type should actually be `[Nat8]`.
  Note: Be sure to explode each `Nat8` of a `Nat32` into its own `Nat32`, and to shift into lower 8 bits.
  // should this really be public?
  */
  public let hashNat8s : [Hash] -> Hash = func(key) {
    var hash = Prim.natToNat32(0);
    for natOfKey in key.values() {
      hash := hash +% natOfKey;
      hash := hash +% hash << 10;
      hash := hash ^ (hash >> 6);
    };
    hash := hash +% hash << 3;
    hash := hash ^ (hash >> 11);
    hash := hash +% hash << 15;
    return hash;
  };

}
