/// 64-bit unsigned integers with checked arithmetic
///
/// Most operations are available as built-in operators (e.g. `1 + 1`).
import Nat "Nat";
import Prim "mo:⛔";

module {

  /// 64-bit natural numbers.
  public type Nat64 = Prim.Types.Nat64;

  /// Conversion.
  public let toNat : Nat64 -> Nat = Prim.nat64ToNat;

  /// Conversion. Traps on overflow/underflow.
  public let fromNat : Nat -> Nat64  = Prim.natToNat64;

  /// Conversion. Wraps on overflow/underflow.
  public let fromIntWrap : Int -> Nat64  = Prim.intToNat64Wrap;

  /// Returns the Text representation of `x`.
  public func toText(x : Nat64) : Text {
    Nat.toText(toNat(x))
  };

  /// Returns the minimum of `x` and `y`.
  public func min(x : Nat64, y : Nat64) : Nat64 {
    if x < y { x } else { y }
  };

  /// Returns the maximum of `x` and `y`.
  public func max(x : Nat64, y : Nat64) : Nat64 {
    if x < y { y } else { x }
  };

  /// Returns `x == y`.
  public func equal(x : Nat64, y : Nat64) : Bool { x == y };

  /// Returns `x != y`.
  public func notEqual(x : Nat64, y : Nat64) : Bool { x != y };

  /// Returns `x < y`.
  public func less(x : Nat64, y : Nat64) : Bool { x < y };

  /// Returns `x <= y`.
  public func lessOrEqual(x : Nat64, y : Nat64) : Bool { x <= y };

  /// Returns `x > y`.
  public func greater(x : Nat64, y : Nat64) : Bool { x > y };

  /// Returns `x >= y`.
  public func greaterOrEqual(x : Nat64, y : Nat64) : Bool { x >= y };

  /// Returns the order of `x` and `y`.
  public func compare(x : Nat64, y : Nat64) : { #less; #equal; #greater } {
    if x < y { #less }
    else if x == y { #equal }
    else { #greater }
  };

  /// Returns the sum of `x` and `y`, `x + y`. Traps on overflow.
  public func add(x : Nat64, y : Nat64) : Nat64 { x + y };

  /// Returns the difference of `x` and `y`, `x - y`. Traps on underflow.
  public func sub(x : Nat64, y : Nat64) : Nat64 { x - y };

  /// Returns the product of `x` and `y`, `x * y`. Traps on overflow.
  public func mul(x : Nat64, y : Nat64) : Nat64 { x * y };

  /// Returns the division of `x by y`, `x / y`.
  /// Traps when `y` is zero.
  public func div(x : Nat64, y : Nat64) : Nat64 { x / y };

  /// Returns the remainder of `x` divided by `y`, `x % y`.
  /// Traps when `y` is zero.
  public func rem(x : Nat64, y : Nat64) : Nat64 { x % y };

  /// Returns `x` to the power of `y`, `x ** y`. Traps on overflow.
  public func pow(x : Nat64, y : Nat64) : Nat64 { x ** y };

  /// Returns the bitwise negation of `x`, `^x`.
  public func bitnot(x : Nat64) : Nat64 { ^x };

  /// Returns the bitwise and of `x` and `y`, `x & y`.
  public func bitand(x : Nat64, y : Nat64) : Nat64 { x & y };

  /// Returns the bitwise or of `x` and `y`, `x \| y`.
  public func bitor(x : Nat64, y : Nat64) : Nat64 { x | y };

  /// Returns the bitwise exclusive or of `x` and `y`, `x ^ y`.
  public func bitxor(x : Nat64, y : Nat64) : Nat64 { x ^ y };

  /// Returns the bitwise shift left of `x` by `y`, `x << y`.
  public func bitshiftLeft(x : Nat64, y : Nat64) : Nat64 { x << y };

  /// Returns the bitwise shift right of `x` by `y`, `x >> y`.
  public func bitshiftRight(x : Nat64, y : Nat64) : Nat64 { x >> y };

  /// Returns the bitwise rotate left of `x` by `y`, `x <<> y`.
  public func bitrotLeft(x : Nat64, y : Nat64) : Nat64 { x <<> y };

  /// Returns the bitwise rotate right of `x` by `y`, `x <>> y`.
  public func bitrotRight(x : Nat64, y : Nat64) : Nat64 { x <>> y };

  /// Returns the value of bit `p mod 64` in `x`, `(x & 2^(p mod 64)) == 2^(p mod 64)`.
  public func bittest(x : Nat64, p : Nat) : Bool {
    Prim.btstNat64(x, Prim.natToNat64(p));
  };

  /// Returns the value of setting bit `p mod 64` in `x` to `1`.
  public func bitset(x : Nat64, p : Nat) : Nat64 {
    x | (1 << Prim.natToNat64(p));
  };

  /// Returns the value of clearing bit `p mod 64` in `x` to `0`.
  public func bitclear(x : Nat64, p : Nat) : Nat64 {
    x & ^(1 << Prim.natToNat64(p));
  };

  /// Returns the value of flipping bit `p mod 64` in `x`.
  public func bitflip(x : Nat64, p : Nat) : Nat64 {
    x ^ (1 << Prim.natToNat64(p));
  };

  /// Returns the count of non-zero bits in `x`.
  public let bitcountNonZero : (x : Nat64) -> Nat64 = Prim.popcntNat64;

  /// Returns the count of leading zero bits in `x`.
  public let bitcountLeadingZero : (x : Nat64) -> Nat64 = Prim.clzNat64;

  /// Returns the count of trailing zero bits in `x`.
  public let bitcountTrailingZero : (x : Nat64) -> Nat64 = Prim.ctzNat64;

  /// Returns the sum of `x` and `y`, `x +% y`. Wraps on overflow.
  public func addWrap(x : Nat64, y : Nat64) : Nat64 { x +% y };

  /// Returns the difference of `x` and `y`, `x -% y`. Wraps on underflow.
  public func subWrap(x : Nat64, y : Nat64) : Nat64 { x -% y };

  /// Returns the product of `x` and `y`, `x *% y`. Wraps on overflow.
  public func mulWrap(x : Nat64, y : Nat64) : Nat64 { x *% y };

  /// Returns `x` to the power of `y`, `x **% y`. Wraps on overflow.
  public func powWrap(x : Nat64, y : Nat64) : Nat64 { x **% y };

}
