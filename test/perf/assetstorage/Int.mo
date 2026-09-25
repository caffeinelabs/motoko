/// Signed integer numbers with infinite precision (also called big integers).
///
/// Most operations on integer numbers (e.g. addition) are available as built-in operators (e.g. `-1 + 1`).
/// This module provides equivalent functions and `Text` conversion.
///
/// Import from the base library to use this module.
/// ```motoko name=import
/// import Int "mo:base/Int";
/// ```

import Prim "mo:⛔";
import Prelude "Prelude";
import Hash "Hash";

module {

  /// Infinite precision signed integers.
  public type Int = Prim.Types.Int;

  /// Returns the absolute value of `x`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.abs(-12) // => 12
  /// ```
  public func abs(x : Int) : Nat {
    Prim.abs(x)
  };

  /// Converts an integer number to its textual representation. Textual
  /// representation _do not_ contain underscores to represent commas.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.toText(-1234) // => "-1234"
  /// ```
  public func toText(x : Int) : Text {
    if x == 0 {
      return "0"
    };

    let isNegative = x < 0;
    var int = if isNegative { -x } else { x };

    var text = "";
    let base = 10;

    while int > 0 {
      let rem = int % base;
      text := (
        switch rem {
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
        }
      ) # text;
      int := int / base
    };

    return if isNegative { "-" # text } else { text }
  };

  /// Returns the minimum of `x` and `y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.min(2, -3) // => -3
  /// ```
  public func min(x : Int, y : Int) : Int {
    if x < y { x } else { y }
  };

  /// Returns the maximum of `x` and `y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.max(2, -3) // => 2
  /// ```
  public func max(x : Int, y : Int) : Int {
    if x < y { y } else { x }
  };

  // this is a local copy of deprecated Hash.hashNat8 (redefined to suppress the warning)
  private func hashNat8(key : [Nat32]) : Hash.Hash {
    var hash : Nat32 = 0;
    for natOfKey in key.values() {
      hash := hash +% natOfKey;
      hash := hash +% hash << 10;
      hash := hash ^ (hash >> 6)
    };
    hash := hash +% hash << 3;
    hash := hash ^ (hash >> 11);
    hash := hash +% hash << 15;
    return hash
  };

  /// Computes a hash from the least significant 32-bits of `i`, ignoring other bits.
  /// @deprecated For large `Int` values consider using a bespoke hash function that considers all of the argument's bits.
  public func hash(i : Int) : Hash.Hash {
    // CAUTION: This removes the high bits!
    let j = Prim.int32ToNat32(Prim.intToInt32Wrap(i));
    hashNat8([
      j & (255 << 0),
      j & (255 << 8),
      j & (255 << 16),
      j & (255 << 24)
    ])
  };

  /// Computes an accumulated hash from `h1` and the least significant 32-bits of `i`, ignoring other bits in `i`.
  /// @deprecated For large `Int` values consider using a bespoke hash function that considers all of the argument's bits.
  public func hashAcc(h1 : Hash.Hash, i : Int) : Hash.Hash {
    // CAUTION: This removes the high bits!
    let j = Prim.int32ToNat32(Prim.intToInt32Wrap(i));
    hashNat8([
      h1,
      j & (255 << 0),
      j & (255 << 8),
      j & (255 << 16),
      j & (255 << 24)
    ])
  };

  /// Equality function for Int types.
  /// This is equivalent to `x == y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.equal(-1, -1); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `==` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `==`
  /// as a function value at the moment.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Buffer "mo:base/Buffer";
  ///
  /// let buffer1 = Buffer.Buffer<Int>(1);
  /// buffer1.add(-3);
  /// let buffer2 = Buffer.Buffer<Int>(1);
  /// buffer2.add(-3);
  /// Buffer.equal(buffer1, buffer2, Int.equal) // => true
  /// ```
  public func equal(x : Int, y : Int) : Bool { x == y };

  /// Inequality function for Int types.
  /// This is equivalent to `x != y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.notEqual(-1, -2); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `!=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `!=`
  /// as a function value at the moment.
  public func notEqual(x : Int, y : Int) : Bool { x != y };

  /// "Less than" function for Int types.
  /// This is equivalent to `x < y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.less(-2, 1); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `<` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `<`
  /// as a function value at the moment.
  public func less(x : Int, y : Int) : Bool { x < y };

  /// "Less than or equal" function for Int types.
  /// This is equivalent to `x <= y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.lessOrEqual(-2, 1); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `<=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `<=`
  /// as a function value at the moment.
  public func lessOrEqual(x : Int, y : Int) : Bool { x <= y };

  /// "Greater than" function for Int types.
  /// This is equivalent to `x > y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.greater(1, -2); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `>` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `>`
  /// as a function value at the moment.
  public func greater(x : Int, y : Int) : Bool { x > y };

  /// "Greater than or equal" function for Int types.
  /// This is equivalent to `x >= y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.greaterOrEqual(1, -2); // => true
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `>=` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `>=`
  /// as a function value at the moment.
  public func greaterOrEqual(x : Int, y : Int) : Bool { x >= y };

  /// General-purpose comparison function for `Int`. Returns the `Order` (
  /// either `#less`, `#equal`, or `#greater`) of comparing `x` with `y`.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.compare(-3, 2) // => #less
  /// ```
  ///
  /// This function can be used as value for a high order function, such as a sort function.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Array "mo:base/Array";
  /// Array.sort([1, -2, -3], Int.compare) // => [-3, -2, 1]
  /// ```
  public func compare(x : Int, y : Int) : { #less; #equal; #greater } {
    if x < y { #less } else if x == y { #equal } else { #greater }
  };

  /// Returns the negation of `x`, `-x` .
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.neg(123) // => -123
  /// ```
  public func neg(x : Int) : Int { -x };

  /// Returns the sum of `x` and `y`, `x + y`.
  ///
  /// No overflow since `Int` has infinite precision.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.add(1, -2); // => -1
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `+` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `+`
  /// as a function value at the moment.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Array "mo:base/Array";
  /// Array.foldLeft([1, -2, -3], 0, Int.add) // => -4
  /// ```
  public func add(x : Int, y : Int) : Int { x + y };

  /// Returns the difference of `x` and `y`, `x - y`.
  ///
  /// No overflow since `Int` has infinite precision.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.sub(1, 2); // => -1
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `-` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `-`
  /// as a function value at the moment.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Array "mo:base/Array";
  /// Array.foldLeft([1, -2, -3], 0, Int.sub) // => 4
  /// ```
  public func sub(x : Int, y : Int) : Int { x - y };

  /// Returns the product of `x` and `y`, `x * y`.
  ///
  /// No overflow since `Int` has infinite precision.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.mul(-2, 3); // => -6
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `*` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `*`
  /// as a function value at the moment.
  ///
  /// Example:
  /// ```motoko include=import
  /// import Array "mo:base/Array";
  /// Array.foldLeft([1, -2, -3], 1, Int.mul) // => 6
  /// ```
  public func mul(x : Int, y : Int) : Int { x * y };

  /// Returns the signed integer division of `x` by `y`,  `x / y`.
  /// Rounds the quotient towards zero, which is the same as truncating the decimal places of the quotient.
  ///
  /// Traps when `y` is zero.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.div(6, -2); // => -3
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `/` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `/`
  /// as a function value at the moment.
  public func div(x : Int, y : Int) : Int { x / y };

  /// Returns the remainder of the signed integer division of `x` by `y`, `x % y`,
  /// which is defined as `x - x / y * y`.
  ///
  /// Traps when `y` is zero.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.rem(6, -4); // => 2
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `%` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `%`
  /// as a function value at the moment.
  public func rem(x : Int, y : Int) : Int { x % y };

  /// Returns `x` to the power of `y`, `x ** y`.
  ///
  /// Traps when `y` is negative or `y > 2 ** 32 - 1`.
  /// No overflow since `Int` has infinite precision.
  ///
  /// Example:
  /// ```motoko include=import
  /// Int.pow(-2, 3); // => -8
  /// ```
  ///
  /// Note: The reason why this function is defined in this library (in addition
  /// to the existing `**` operator) is so that you can use it as a function
  /// value to pass to a higher order function. It is not possible to use `**`
  /// as a function value at the moment.
  public func pow(x : Int, y : Int) : Int { x ** y };

}
