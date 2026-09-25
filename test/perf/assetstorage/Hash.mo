/// Hash values

import Prim "mo:⛔";
import Iter "Iter";

module {
  /// Hash values represent a string of _hash bits_, packed into a `Nat32`.
  public type Hash = Nat32;

  /// The hash length, always 31.
  public let length : Nat = 31; // Why not 32?

  /// Project a given bit from the bit vector.
  public func bit(h : Hash, pos : Nat) : Bool {
    assert (pos <= length);
    (h & (Prim.natToNat32(1) << Prim.natToNat32(pos))) != Prim.natToNat32(0)
  };

  /// Test if two hashes are equal
  public func equal(ha : Hash, hb : Hash) : Bool {
    ha == hb
  };

  public func hash(i : Nat) : Hash {
    let j = Prim.natToNat32(i);
    hashNat8(
      [
        j & (255 << 0),
        j & (255 << 8),
        j & (255 << 16),
        j & (255 << 24)
      ]
    );
  };

  public func debugPrintBits(bits : Hash) {
    for j in Iter.range(0, length - 1) {
      if bit(bits, j) {
        Prim.debugPrint("1")
      } else {
        Prim.debugPrint("0")
      }
    }
  };

  public func debugPrintBitsRev(bits : Hash) {
    for j in Iter.revRange(length - 1, 0) {
      if bit(bits, Prim.abs(j)) {
        Prim.debugPrint("1")
      } else {
        Prim.debugPrint("0")
      }
    }
  };

  /// Jenkin's one at a time:
  ///
  /// https://en.wikipedia.org/wiki/Jenkins_hash_function#one_at_a_time
  ///
  /// The input type should actually be `[Nat8]`.
  /// Note: Be sure to explode each `Nat8` of a `Nat32` into its own `Nat32`, and to shift into lower 8 bits.

  // should this really be public?
  public func hashNat8(key : [Hash]) : Hash {
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
