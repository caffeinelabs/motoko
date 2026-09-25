/// 8-bit unsigned integers with checked arithmetic
///
/// Most operations are available as built-in operators (e.g. `1 + 1`).
import Nat "Nat";
import Prim "mo:⛔";

module {

  /// 8-bit natural numbers.
  public type Nat8 = Prim.Types.Nat8;

  /// Conversion.
  public let toNat : Nat8 -> Nat = Prim.nat8ToNat;

  /// Conversion. Traps on overflow/underflow.
  public let fromNat : Nat -> Nat8  = Prim.natToNat8;

  /// Conversion. Wraps on overflow/underflow.
  public let fromIntWrap : Int -> Nat8  = Prim.intToNat8Wrap;

  /// Returns the Text representation of `x`.
  public func toText(x : Nat8) : Text {
    Nat.toText(toNat(x))
  };

  /// Returns the minimum of `x` and `y`.
  public func min(x : Nat8, y : Nat8) : Nat8 {
    if x < y { x } else { y }
  };

  /// Returns the maximum of `x` and `y`.
  public func max(x : Nat8, y : Nat8) : Nat8 {
    if x < y { y } else { x }
  };

  /// Returns `x == y`.
  public func equal(x : Nat8, y : Nat8) : Bool { x == y };

  /// Returns `x != y`.
  public func notEqual(x : Nat8, y : Nat8) : Bool { x != y };

  /// Returns `x < y`.
  public func less(x : Nat8, y : Nat8) : Bool { x < y };

  /// Returns `x <= y`.
  public func lessOrEqual(x : Nat8, y : Nat8) : Bool { x <= y };

  /// Returns `x > y`.
  public func greater(x : Nat8, y : Nat8) : Bool { x > y };

  /// Returns `x >= y`.
  public func greaterOrEqual(x : Nat8, y : Nat8) : Bool { x >= y };

  /// Returns the order of `x` and `y`.
  public func compare(x : Nat8, y : Nat8) : { #less; #equal; #greater } {
    if x < y { #less }
    else if x == y { #equal }
    else { #greater }
  };

  /// Returns the sum of `x` and `y`, `x + y`. Traps on overflow.
  public func add(x : Nat8, y : Nat8) : Nat8 { x + y };

  /// Returns the difference of `x` and `y`, `x - y`. Traps on underflow.
  public func sub(x : Nat8, y : Nat8) : Nat8 { x - y };

  /// Returns the product of `x` and `y`, `x * y`. Traps on overflow.
  public func mul(x : Nat8, y : Nat8) : Nat8 { x * y };

  /// Returns the division of `x by y`, `x / y`.
  /// Traps when `y` is zero.
  public func div(x : Nat8, y : Nat8) : Nat8 { x / y };

  /// Returns the remainder of `x` divided by `y`, `x % y`.
  /// Traps when `y` is zero.
  public func rem(x : Nat8, y : Nat8) : Nat8 { x % y };

  /// Returns `x` to the power of `y`, `x ** y`. Traps on overflow.
  public func pow(x : Nat8, y : Nat8) : Nat8 { x ** y };

  /// Returns the bitwise negation of `x`, `^x`.
  public func bitnot(x : Nat8) : Nat8 { ^x };

  /// Returns the bitwise and of `x` and `y`, `x & y`.
  public func bitand(x : Nat8, y : Nat8) : Nat8 { x & y };

  /// Returns the bitwise or of `x` and `y`, `x \| y`.
  public func bitor(x : Nat8, y : Nat8) : Nat8 { x | y };

  /// Returns the bitwise exclusive or of `x` and `y`, `x ^ y`.
  public func bitxor(x : Nat8, y : Nat8) : Nat8 { x ^ y };

  /// Returns the bitwise shift left of `x` by `y`, `x << y`.
  public func bitshiftLeft(x : Nat8, y : Nat8) : Nat8 { x << y };

  /// Returns the bitwise shift right of `x` by `y`, `x >> y`.
  public func bitshiftRight(x : Nat8, y : Nat8) : Nat8 { x >> y };

  /// Returns the bitwise rotate left of `x` by `y`, `x <<> y`.
  public func bitrotLeft(x : Nat8, y : Nat8) : Nat8 { x <<> y };

  /// Returns the bitwise rotate right of `x` by `y`, `x <>> y`.
  public func bitrotRight(x : Nat8, y : Nat8) : Nat8 { x <>> y };

  /// Returns the value of bit `p mod 8` in `x`, `(x & 2^(p mod 8)) == 2^(p mod 8)`.
  public func bittest(x : Nat8, p : Nat) : Bool {
    Prim.btstNat8(x, Prim.natToNat8(p));
  };

  /// Returns the value of setting bit `p mod 8` in `x` to `1`.
  public func bitset(x : Nat8, p : Nat) : Nat8 {
    x | (1 << Prim.natToNat8(p));
  };

  /// Returns the value of clearing bit `p mod 8` in `x` to `0`.
  public func bitclear(x : Nat8, p : Nat) : Nat8 {
    x & ^(1 << Prim.natToNat8(p));
  };

  /// Returns the value of flipping bit `p mod 8` in `x`.
  public func bitflip(x : Nat8, p : Nat) : Nat8 {
    x ^ (1 << Prim.natToNat8(p));
  };

  /// Returns the count of non-zero bits in `x`.
  public let bitcountNonZero : (x : Nat8) -> Nat8 = Prim.popcntNat8;

  /// Returns the count of leading zero bits in `x`.
  public let bitcountLeadingZero : (x : Nat8) -> Nat8 = Prim.clzNat8;

  /// Returns the count of trailing zero bits in `x`.
  public let bitcountTrailingZero : (x : Nat8) -> Nat8 = Prim.ctzNat8;

  /// Returns the sum of `x` and `y`, `x +% y`. Wraps on overflow.
  public func addWrap(x : Nat8, y : Nat8) : Nat8 { x +% y };

  /// Returns the difference of `x` and `y`, `x -% y`. Wraps on underflow.
  public func subWrap(x : Nat8, y : Nat8) : Nat8 { x -% y };

  /// Returns the product of `x` and `y`, `x *% y`. Wraps on overflow.
  public func mulWrap(x : Nat8, y : Nat8) : Nat8 { x *% y };

  /// Returns `x` to the power of `y`, `x **% y`. Wraps on overflow.
  public func powWrap(x : Nat8, y : Nat8) : Nat8 { x **% y };

}
