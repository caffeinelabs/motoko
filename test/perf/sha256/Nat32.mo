/// 32-bit unsigned integers with checked arithmetic
///
/// Most operations are available as built-in operators (e.g. `1 + 1`).
import Nat "Nat";
import Prim "mo:⛔";

module {

  /// 32-bit natural numbers.
  public type Nat32 = Prim.Types.Nat32;

  /// Conversion.
  public let toNat : Nat32 -> Nat = Prim.nat32ToNat;

  /// Conversion. Traps on overflow/underflow.
  public let fromNat : Nat -> Nat32  = Prim.natToNat32;

  /// Conversion. Wraps on overflow/underflow.
  public let fromIntWrap : Int -> Nat32  = Prim.intToNat32Wrap;

  /// Returns the Text representation of `x`.
  public func toText(x : Nat32) : Text {
    Nat.toText(toNat(x))
  };

  /// Returns the minimum of `x` and `y`.
  public func min(x : Nat32, y : Nat32) : Nat32 {
    if x < y { x } else { y }
  };

  /// Returns the maximum of `x` and `y`.
  public func max(x : Nat32, y : Nat32) : Nat32 {
    if x < y { y } else { x }
  };

  /// Returns `x == y`.
  public func equal(x : Nat32, y : Nat32) : Bool { x == y };

  /// Returns `x != y`.
  public func notEqual(x : Nat32, y : Nat32) : Bool { x != y };

  /// Returns `x < y`.
  public func less(x : Nat32, y : Nat32) : Bool { x < y };

  /// Returns `x <= y`.
  public func lessOrEqual(x : Nat32, y : Nat32) : Bool { x <= y };

  /// Returns `x > y`.
  public func greater(x : Nat32, y : Nat32) : Bool { x > y };

  /// Returns `x >= y`.
  public func greaterOrEqual(x : Nat32, y : Nat32) : Bool { x >= y };

  /// Returns the order of `x` and `y`.
  public func compare(x : Nat32, y : Nat32) : { #less; #equal; #greater } {
    if x < y { #less }
    else if x == y { #equal }
    else { #greater }
  };

  /// Returns the sum of `x` and `y`, `x + y`. Traps on overflow.
  public func add(x : Nat32, y : Nat32) : Nat32 { x + y };

  /// Returns the difference of `x` and `y`, `x - y`. Traps on underflow.
  public func sub(x : Nat32, y : Nat32) : Nat32 { x - y };

  /// Returns the product of `x` and `y`, `x * y`. Traps on overflow.
  public func mul(x : Nat32, y : Nat32) : Nat32 { x * y };

  /// Returns the division of `x by y`, `x / y`.
  /// Traps when `y` is zero.
  public func div(x : Nat32, y : Nat32) : Nat32 { x / y };

  /// Returns the remainder of `x` divided by `y`, `x % y`.
  /// Traps when `y` is zero.
  public func rem(x : Nat32, y : Nat32) : Nat32 { x % y };

  /// Returns `x` to the power of `y`, `x ** y`. Traps on overflow.
  public func pow(x : Nat32, y : Nat32) : Nat32 { x ** y };

  /// Returns the bitwise negation of `x`, `^x`.
  public func bitnot(x : Nat32) : Nat32 { ^x };

  /// Returns the bitwise and of `x` and `y`, `x & y`.
  public func bitand(x : Nat32, y : Nat32) : Nat32 { x & y };

  /// Returns the bitwise or of `x` and `y`, `x \| y`.
  public func bitor(x : Nat32, y : Nat32) : Nat32 { x | y };

  /// Returns the bitwise exclusive or of `x` and `y`, `x ^ y`.
  public func bitxor(x : Nat32, y : Nat32) : Nat32 { x ^ y };

  /// Returns the bitwise shift left of `x` by `y`, `x << y`.
  public func bitshiftLeft(x : Nat32, y : Nat32) : Nat32 { x << y };

  /// Returns the bitwise shift right of `x` by `y`, `x >> y`.
  public func bitshiftRight(x : Nat32, y : Nat32) : Nat32 { x >> y };

  /// Returns the bitwise rotate left of `x` by `y`, `x <<> y`.
  public func bitrotLeft(x : Nat32, y : Nat32) : Nat32 { x <<> y };

  /// Returns the bitwise rotate right of `x` by `y`, `x <>> y`.
  public func bitrotRight(x : Nat32, y : Nat32) : Nat32 { x <>> y };

  /// Returns the value of bit `p mod 32` in `x`, `(x & 2^(p mod 32)) == 2^(p mod 32)`.
  public func bittest(x : Nat32, p : Nat) : Bool {
    Prim.btstNat32(x, Prim.natToNat32(p));
  };

  /// Returns the value of setting bit `p mod 32` in `x` to `1`.
  public func bitset(x : Nat32, p : Nat) : Nat32 {
    x | (1 << Prim.natToNat32(p));
  };

  /// Returns the value of clearing bit `p mod 32` in `x` to `0`.
  public func bitclear(x : Nat32, p : Nat) : Nat32 {
    x & ^(1 << Prim.natToNat32(p));
  };

  /// Returns the value of flipping bit `p mod 32` in `x`.
  public func bitflip(x : Nat32, p : Nat) : Nat32 {
    x ^ (1 << Prim.natToNat32(p));
  };

  /// Returns the count of non-zero bits in `x`.
  public let bitcountNonZero : (x : Nat32) -> Nat32 = Prim.popcntNat32;

  /// Returns the count of leading zero bits in `x`.
  public let bitcountLeadingZero : (x : Nat32) -> Nat32 = Prim.clzNat32;

  /// Returns the count of trailing zero bits in `x`.
  public let bitcountTrailingZero : (x : Nat32) -> Nat32 = Prim.ctzNat32;

  /// Returns the sum of `x` and `y`, `x +% y`. Wraps on overflow.
  public func addWrap(x : Nat32, y : Nat32) : Nat32 { x +% y };

  /// Returns the difference of `x` and `y`, `x -% y`. Wraps on underflow.
  public func subWrap(x : Nat32, y : Nat32) : Nat32 { x -% y };

  /// Returns the product of `x` and `y`, `x *% y`. Wraps on overflow.
  public func mulWrap(x : Nat32, y : Nat32) : Nat32 { x *% y };

  /// Returns `x` to the power of `y`, `x **% y`. Wraps on overflow.
  public func powWrap(x : Nat32, y : Nat32) : Nat32 { x **% y };

}
