/// Integer numbers
///
/// Most operations on integers (e.g. addition) are available as built-in operators (e.g. `1 + 1`).
/// This module provides equivalent functions and `Text` conversion.

import Prim "mo:⛔";
import Prelude "Prelude";
import Hash "Hash";

module {

  /// Infinite precision signed integers.
  public type Int = Prim.Types.Int;

  /// Returns the absolute value of the number
  public let abs : Int -> Nat = Prim.abs;

  /// Conversion.
  public let toText : Int -> Text = func(x) {
    if x == 0 {
      return "0";
    };

    let isNegative = x < 0;
    var int = if isNegative { -x } else { x };

    var text = "";
    let base = 10;

    while int > 0 {
      let rem = int % base;
      text := (switch rem {
        case 0 { "0" }
        case 1 { "1" }
        case 2 { "2" }
        case 3 { "3" }
        case 4 { "4" }
        case 5 { "5" }
        case 6 { "6" }
        case 7 { "7" }
        case 8 { "8" }
        case 9 { "9" }
        case _ { Prelude.unreachable() }
      }) # text;
      int := int / base;
    };

    return if isNegative { "-" # text } else { text };
  };

  /// Returns the minimum of `x` and `y`.
  public func min(x : Int, y : Int) : Int {
    if x < y { x } else { y };
  };

  /// Returns the maximum of `x` and `y`.
  public func max(x : Int, y : Int) : Int {
    if x < y { y } else { x };
  };

  /// Returns `x == y`.
  public func equal(x : Int, y : Int) : Bool { x == y };

  /// Returns `x != y`.
  public func notEqual(x : Int, y : Int) : Bool { x != y };

  /// Returns `x < y`.
  public func less(x : Int, y : Int) : Bool { x < y };

  /// Returns `x <= y`.
  public func lessOrEqual(x : Int, y : Int) : Bool { x <= y };

  /// Returns `x > y`.
  public func greater(x : Int, y : Int) : Bool { x > y };

  /// Returns `x >= y`.
  public func greaterOrEqual(x : Int, y : Int) : Bool { x >= y };

  /// Returns the order of `x` and `y`.
  public func compare(x : Int, y : Int) : { #less; #equal; #greater } {
    if x < y { #less }
    else if x == y { #equal }
    else { #greater }
  };

  /// Returns the negation of `x`, `-x` .
  public func neq(x : Int) : Int { -x; };

  /// Returns the sum of `x` and `y`, `x + y`.
  public func add(x : Int, y : Int) : Int { x + y };

  /// Returns the difference of `x` and `y`, `x - y`.
  public func sub(x : Int, y : Int) : Int { x - y };

  /// Returns the product of `x` and `y`, `x * y`.
  public func mul(x : Int, y : Int) : Int { x * y };

  /// Returns the division of `x` by `y`,  `x / y`.
  /// Traps when `y` is zero.
  public func div(x : Int, y : Int) : Int { x / y };

  /// Returns the remainder of `x` divided by `y`, `x % y`.
  /// Traps when `y` is zero.
  public func rem(x : Int, y : Int) : Int { x % y };

  /// Returns `x` to the power of `y`, `x ** y`.
  public func pow(x : Int, y : Int) : Int { x ** y };

}
