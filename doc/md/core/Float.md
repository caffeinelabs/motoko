# core/Float
Double precision (64-bit) floating-point numbers in IEEE 754 representation.

This module contains common floating-point constants and utility functions.

```motoko name=import
import Float "mo:core/Float";
```

Notation for special values in the documentation below:
`+inf`: Positive infinity
`-inf`: Negative infinity
`NaN`: "not a number" (can have different sign bit values, but `NaN != NaN` regardless of the sign).

Note:
Floating point numbers have limited precision and operations may inherently result in numerical errors.

Examples of numerical errors:
  ```motoko
  assert 0.1 + 0.1 + 0.1 != 0.3;
  ```

  ```motoko
  assert not (1e16 + 1.0 != 1e16);
  ```

 (and many more cases)

Advice:
* Floating point number comparisons by `==` or `!=` are discouraged. Instead, it is better to compare
  floating-point numbers with a numerical tolerance, called epsilon.

  Example:
  ```motoko
  import Float "mo:core/Float";
  let x = 0.1 + 0.1 + 0.1;
  let y = 0.3;

  let epsilon = 1e-6; // This depends on the application case (needs a numerical error analysis).
  assert Float.equal(x, y, epsilon);
  ```

* For absolute precision, it is recommened to encode the fraction number as a pair of a Nat for the base
  and a Nat for the exponent (decimal point).

NaN sign:
* The NaN sign is only applied by `abs`, `neg`, and `copySign`. Other operations can have an arbitrary
  sign bit for NaN results.

## Type `Float`
``` motoko no-repl
type Float = Prim.Types.Float
```

64-bit floating point number type.

## Value `pi`
``` motoko no-repl
let pi : Float
```

Ratio of the circumference of a circle to its diameter.
Note: Limited precision.

## Value `e`
``` motoko no-repl
let e : Float
```

Base of the natural logarithm.
Note: Limited precision.

## Function `isNaN`
``` motoko no-repl
func isNaN(self : Float) : Bool
```

Determines whether the `number` is a `NaN` ("not a number" in the floating point representation).
Notes:
* Equality test of `NaN` with itself or another number is always `false`.
* There exist many internal `NaN` value representations, such as positive and negative NaN,
  signalling and quiet NaNs, each with many different bit representations.

Example:
```motoko include=import
assert Float.isNaN(0.0/0.0);
```

## Function `abs`
``` motoko no-repl
func abs(x : Float) : Float
```

Returns the absolute value of `x`.

Special cases:
```
abs(+inf) => +inf
abs(-inf) => +inf
abs(-NaN)  => +NaN
abs(-0.0) => 0.0
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.abs(-1.2), 1.2, epsilon);
```

## Function `sqrt`
``` motoko no-repl
func sqrt(x : Float) : Float
```

Returns the square root of `x`.

Special cases:
```
sqrt(+inf) => +inf
sqrt(-0.0) => -0.0
sqrt(x)    => NaN if x < 0.0
sqrt(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.sqrt(6.25), 2.5, epsilon);
```

## Function `ceil`
``` motoko no-repl
func ceil(x : Float) : Float
```

Returns the smallest integral float greater than or equal to `x`.

Special cases:
```
ceil(+inf) => +inf
ceil(-inf) => -inf
ceil(NaN)  => NaN
ceil(0.0)  => 0.0
ceil(-0.0) => -0.0
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.ceil(1.2), 2.0, epsilon);
```

## Function `floor`
``` motoko no-repl
func floor(x : Float) : Float
```

Returns the largest integral float less than or equal to `x`.

Special cases:
```
floor(+inf) => +inf
floor(-inf) => -inf
floor(NaN)  => NaN
floor(0.0)  => 0.0
floor(-0.0) => -0.0
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.floor(1.2), 1.0, epsilon);
```

## Function `trunc`
``` motoko no-repl
func trunc(x : Float) : Float
```

Returns the nearest integral float not greater in magnitude than `x`.
This is equivalent to returning `x` with truncating its decimal places.

Special cases:
```
trunc(+inf) => +inf
trunc(-inf) => -inf
trunc(NaN)  => NaN
trunc(0.0)  => 0.0
trunc(-0.0) => -0.0
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.trunc(2.75), 2.0, epsilon);
```

## Function `nearest`
``` motoko no-repl
func nearest(x : Float) : Float
```

Returns the nearest integral float to `x`.
A decimal place of exactly .5 is rounded to the nearest even integral float.
and rounded down for `x < 0`

Special cases:
```
nearest(+inf) => +inf
nearest(-inf) => -inf
nearest(NaN)  => NaN
nearest(0.0)  => 0.0
nearest(-0.0) => -0.0
nearest(14.5) => 14.0
```

Example:
```motoko include=import
assert Float.nearest(2.75) == 3.0
```

## Function `copySign`
``` motoko no-repl
func copySign(x : Float, y : Float) : Float
```

Returns `x` if `x` and `y` have same sign, otherwise `x` with negated sign.

The sign bit of zero, infinity, and `NaN` is considered.

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.copySign(1.2, -2.3), -1.2, epsilon);
```

## Function `min`
``` motoko no-repl
func min(x : Float, y : Float) : Float
```

Returns the smaller value of `x` and `y`.

Special cases:
```
min(NaN, y) => NaN for any Float y
min(x, NaN) => NaN for any Float x
```

Example:
```motoko include=import
assert Float.min(1.2, -2.3) == -2.3; // with numerical imprecision
```

## Function `max`
``` motoko no-repl
func max(x : Float, y : Float) : Float
```

Returns the larger value of `x` and `y`.

Special cases:
```
max(NaN, y) => NaN for any Float y
max(x, NaN) => NaN for any Float x
```

Example:
```motoko include=import
assert Float.max(1.2, -2.3) == 1.2;
```

## Function `sin`
``` motoko no-repl
func sin(x : Float) : Float
```

Returns the sine of the radian angle `x`.

Special cases:
```
sin(+inf) => NaN
sin(-inf) => NaN
sin(NaN) => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.sin(Float.pi / 2), 1.0, epsilon);
```

## Function `cos`
``` motoko no-repl
func cos(x : Float) : Float
```

Returns the cosine of the radian angle `x`.

Special cases:
```
cos(+inf) => NaN
cos(-inf) => NaN
cos(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.cos(Float.pi / 2), 0.0, epsilon);
```

## Function `tan`
``` motoko no-repl
func tan(x : Float) : Float
```

Returns the tangent of the radian angle `x`.

Special cases:
```
tan(+inf) => NaN
tan(-inf) => NaN
tan(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.tan(Float.pi / 4), 1.0, epsilon);
```

## Function `arcsin`
``` motoko no-repl
func arcsin(x : Float) : Float
```

Returns the arc sine of `x` in radians.

Special cases:
```
arcsin(x)   => NaN if x > 1.0
arcsin(x)   => NaN if x < -1.0
arcsin(NaN) => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.arcsin(1.0), Float.pi / 2, epsilon);
```

## Function `arccos`
``` motoko no-repl
func arccos(x : Float) : Float
```

Returns the arc cosine of `x` in radians.

Special cases:
```
arccos(x)  => NaN if x > 1.0
arccos(x)  => NaN if x < -1.0
arcos(NaN) => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.arccos(1.0), 0.0, epsilon);
```

## Function `arctan`
``` motoko no-repl
func arctan(x : Float) : Float
```

Returns the arc tangent of `x` in radians.

Special cases:
```
arctan(+inf) => pi / 2
arctan(-inf) => -pi / 2
arctan(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.arctan(1.0), Float.pi / 4, epsilon);
```

## Function `arctan2`
``` motoko no-repl
func arctan2(x : Float, y : Float) : Float
```

Given `(y,x)`, returns the arc tangent in radians of `y/x` based on the signs of both values to determine the correct quadrant.

Special cases:
```
arctan2(0.0, 0.0)   => 0.0
arctan2(-0.0, 0.0)  => -0.0
arctan2(0.0, -0.0)  => pi
arctan2(-0.0, -0.0) => -pi
arctan2(+inf, +inf) => pi / 4
arctan2(+inf, -inf) => 3 * pi / 4
arctan2(-inf, +inf) => -pi / 4
arctan2(-inf, -inf) => -3 * pi / 4
arctan2(NaN, x)     => NaN for any Float x
arctan2(y, NaN)     => NaN for any Float y
```

Example:
```motoko include=import
let sqrt2over2 = Float.sqrt(2) / 2;
assert Float.arctan2(sqrt2over2, sqrt2over2) == Float.pi / 4;
```

## Function `exp`
``` motoko no-repl
func exp(x : Float) : Float
```

Returns the value of `e` raised to the `x`-th power.

Special cases:
```
exp(+inf) => +inf
exp(-inf) => 0.0
exp(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.exp(1.0), Float.e, epsilon);
```

## Function `log`
``` motoko no-repl
func log(x : Float) : Float
```

Returns the natural logarithm (base-`e`) of `x`.

Special cases:
```
log(0.0)  => -inf
log(-0.0) => -inf
log(x)    => NaN if x < 0.0
log(+inf) => +inf
log(NaN)  => NaN
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.log(Float.e), 1.0, epsilon);
```

## Function `format`
``` motoko no-repl
func format(self : Float, fmt : {#fix : Nat8; #exp : Nat8; #gen : Nat8; #exact}) : Text
```

Formatting. `format(fmt, x)` formats `x` to `Text` according to the
formatting directive `fmt`, which can take one of the following forms:

* `#fix prec` as fixed-point format with `prec` digits
* `#exp prec` as exponential format with `prec` digits
* `#gen prec` as generic format with `prec` digits
* `#exact` as exact format that can be decoded without loss.

`-0.0` is formatted with negative sign bit.
Positive infinity is formatted as "inf".
Negative infinity is formatted as "-inf".

The numerical precision and the text format can vary between
Motoko versions and runtime configuration. Moreover, `NaN` can be printed
differently, i.e. "NaN" or "nan", potentially omitting the `NaN` sign.

Example:
```motoko include=import no-validate
assert Float.format(#exp 3, 123.0) == "1.230e+02";
```

## Function `toText`
``` motoko no-repl
func toText(self : Float) : Text
```

Conversion to Text. Use `format(fmt, x)` for more detailed control.

`-0.0` is formatted with negative sign bit.
Positive infinity is formatted as `inf`.
Negative infinity is formatted as `-inf`.
`NaN` is formatted as `NaN` or `-NaN` depending on its sign bit.

The numerical precision and the text format can vary between
Motoko versions and runtime configuration. Moreover, `NaN` can be printed
differently, i.e. "NaN" or "nan", potentially omitting the `NaN` sign.

Example:
```motoko include=import no-validate
assert Float.toText(1.2) == "1.2";
```

## Function `toInt64`
``` motoko no-repl
func toInt64(self : Float) : Int64
```

Conversion to Int64 by truncating Float, equivalent to `toInt64(trunc(f))`

Traps if the floating point number is larger or smaller than the representable Int64.
Also traps for `inf`, `-inf`, and `NaN`.

Example:
```motoko include=import
assert Float.toInt64(-12.3) == -12;
```

## Function `fromInt64`
``` motoko no-repl
func fromInt64(x : Int64) : Float
```

Conversion from Int64.

Note: The floating point number may be imprecise for large or small Int64.

Example:
```motoko include=import
assert Float.fromInt64(-42) == -42.0;
```

## Function `toInt`
``` motoko no-repl
func toInt(self : Float) : Int
```

Conversion to Int.

Traps for `inf`, `-inf`, and `NaN`.

Example:
```motoko include=import
assert Float.toInt(1.2e6) == +1_200_000;
```

## Function `fromInt`
``` motoko no-repl
func fromInt(x : Int) : Float
```

Conversion from Int. May result in `Inf`.

Note: The floating point number may be imprecise for large or small Int values.
Returns `inf` if the integer is greater than the maximum floating point number.
Returns `-inf` if the integer is less than the minimum floating point number.

Example:
```motoko include=import
assert Float.fromInt(-123) == -123.0;
```
@deprecated M0235

## Function `equal`
``` motoko no-repl
func equal(x : Float, y : Float, epsilon : Float) : Bool
```

Determines whether `x` is equal to `y` within the defined tolerance of `epsilon`.
The `epsilon` considers numerical erros, see comment above.
Equivalent to `Float.abs(x - y) <= epsilon` for a non-negative epsilon.

Traps if `epsilon` is negative or `NaN`.

Special cases:
```
equal(+0.0, -0.0, epsilon) => true for any `epsilon >= 0.0`
equal(-0.0, +0.0, epsilon) => true for any `epsilon >= 0.0`
equal(+inf, +inf, epsilon) => true for any `epsilon >= 0.0`
equal(-inf, -inf, epsilon) => true for any `epsilon >= 0.0`
equal(x, NaN, epsilon)     => false for any x and `epsilon >= 0.0`
equal(NaN, y, epsilon)     => false for any y and `epsilon >= 0.0`
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(-12.3, -1.23e1, epsilon);
```

## Function `notEqual`
``` motoko no-repl
func notEqual(x : Float, y : Float, epsilon : Float) : Bool
```

Determines whether `x` is not equal to `y` within the defined tolerance of `epsilon`.
The `epsilon` considers numerical erros, see comment above.
Equivalent to `not equal(x, y, epsilon)`.

Traps if `epsilon` is negative or `NaN`.

Special cases:
```
notEqual(+0.0, -0.0, epsilon) => false for any `epsilon >= 0.0`
notEqual(-0.0, +0.0, epsilon) => false for any `epsilon >= 0.0`
notEqual(+inf, +inf, epsilon) => false for any `epsilon >= 0.0`
notEqual(-inf, -inf, epsilon) => false for any `epsilon >= 0.0`
notEqual(x, NaN, epsilon)     => true for any x and `epsilon >= 0.0`
notEqual(NaN, y, epsilon)     => true for any y and `epsilon >= 0.0`
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert not Float.notEqual(-12.3, -1.23e1, epsilon);
```

## Function `less`
``` motoko no-repl
func less(x : Float, y : Float) : Bool
```

Returns `x < y`.

Special cases:
```
less(+0.0, -0.0) => false
less(-0.0, +0.0) => false
less(NaN, y)     => false for any Float y
less(x, NaN)     => false for any Float x
```

Example:
```motoko include=import
assert Float.less(Float.e, Float.pi);
```

## Function `lessOrEqual`
``` motoko no-repl
func lessOrEqual(x : Float, y : Float) : Bool
```

Returns `x <= y`.

Special cases:
```
lessOrEqual(+0.0, -0.0) => true
lessOrEqual(-0.0, +0.0) => true
lessOrEqual(NaN, y)     => false for any Float y
lessOrEqual(x, NaN)     => false for any Float x
```

Example:
```motoko include=import
assert Float.lessOrEqual(0.123, 0.1234);
```

## Function `greater`
``` motoko no-repl
func greater(x : Float, y : Float) : Bool
```

Returns `x > y`.

Special cases:
```
greater(+0.0, -0.0) => false
greater(-0.0, +0.0) => false
greater(NaN, y)     => false for any Float y
greater(x, NaN)     => false for any Float x
```

Example:
```motoko include=import
assert Float.greater(Float.pi, Float.e);
```

## Function `greaterOrEqual`
``` motoko no-repl
func greaterOrEqual(x : Float, y : Float) : Bool
```

Returns `x >= y`.

Special cases:
```
greaterOrEqual(+0.0, -0.0) => true
greaterOrEqual(-0.0, +0.0) => true
greaterOrEqual(NaN, y)     => false for any Float y
greaterOrEqual(x, NaN)     => false for any Float x
```

Example:
```motoko include=import
assert Float.greaterOrEqual(0.1234, 0.123);
```

## Function `compare`
``` motoko no-repl
func compare(x : Float, y : Float) : Order.Order
```

Defines a total order of `x` and `y` for use in sorting.

Note: Using this operation to determine equality or inequality is discouraged for two reasons:
* It does not consider numerical errors, see comment above. Use `equal(x, y, espilon)` or
  `notEqual(x, y, epsilon)` to test for equality or inequality, respectively.
* `NaN` are here considered equal if their sign matches, which is different to the standard equality
   by `==` or when using `equal()` or `notEqual()`.

Total order:
* negative NaN (no distinction between signalling and quiet negative NaN)
* negative infinity
* negative numbers (including negative subnormal numbers in standard order)
* negative zero (`-0.0`)
* positive zero (`+0.0`)
* positive numbers (including positive subnormal numbers in standard order)
* positive infinity
* positive NaN (no distinction between signalling and quiet positive NaN)

Example:
```motoko include=import
assert Float.compare(0.123, 0.1234) == #less;
```

## Function `neg`
``` motoko no-repl
func neg(x : Float) : Float
```

Returns the negation of `x`, `-x` .

Changes the sign bit for infinity.

Special cases:
```
neg(+inf) => -inf
neg(-inf) => +inf
neg(+NaN) => -NaN
neg(-NaN) => +NaN
neg(+0.0) => -0.0
neg(-0.0) => +0.0
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.neg(1.23), -1.23, epsilon);
```

## Function `add`
``` motoko no-repl
func add(x : Float, y : Float) : Float
```

Returns the sum of `x` and `y`, `x + y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
add(+inf, y)    => +inf if y is any Float except -inf and NaN
add(-inf, y)    => -inf if y is any Float except +inf and NaN
add(+inf, -inf) => NaN
add(NaN, y)     => NaN for any Float y
```
The same cases apply commutatively, i.e. for `add(y, x)`.

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.add(1.23, 0.123), 1.353, epsilon);
```

## Function `sub`
``` motoko no-repl
func sub(x : Float, y : Float) : Float
```

Returns the difference of `x` and `y`, `x - y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
sub(+inf, y)    => +inf if y is any Float except +inf or NaN
sub(-inf, y)    => -inf if y is any Float except -inf and NaN
sub(x, +inf)    => -inf if x is any Float except +inf and NaN
sub(x, -inf)    => +inf if x is any Float except -inf and NaN
sub(+inf, +inf) => NaN
sub(-inf, -inf) => NaN
sub(NaN, y)     => NaN for any Float y
sub(x, NaN)     => NaN for any Float x
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.sub(1.23, 0.123), 1.107, epsilon);
```

## Function `mul`
``` motoko no-repl
func mul(x : Float, y : Float) : Float
```

Returns the product of `x` and `y`, `x * y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
mul(+inf, y) => +inf if y > 0.0
mul(-inf, y) => -inf if y > 0.0
mul(+inf, y) => -inf if y < 0.0
mul(-inf, y) => +inf if y < 0.0
mul(+inf, 0.0) => NaN
mul(-inf, 0.0) => NaN
mul(NaN, y) => NaN for any Float y
```
The same cases apply commutatively, i.e. for `mul(y, x)`.

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.mul(1.23, 1e2), 123.0, epsilon);
```

## Function `div`
``` motoko no-repl
func div(x : Float, y : Float) : Float
```

Returns the division of `x` by `y`, `x / y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
div(0.0, 0.0) => NaN
div(x, 0.0)   => +inf for x > 0.0
div(x, 0.0)   => -inf for x < 0.0
div(x, +inf)  => 0.0 for any x except +inf, -inf, and NaN
div(x, -inf)  => 0.0 for any x except +inf, -inf, and NaN
div(+inf, y)  => +inf if y >= 0.0
div(+inf, y)  => -inf if y < 0.0
div(-inf, y)  => -inf if y >= 0.0
div(-inf, y)  => +inf if y < 0.0
div(NaN, y)   => NaN for any Float y
div(x, NaN)   => NaN for any Float x
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.div(1.23, 1e2), 0.0123, epsilon);
```

## Function `rem`
``` motoko no-repl
func rem(x : Float, y : Float) : Float
```

Returns the floating point division remainder `x % y`,
which is defined as `x - trunc(x / y) * y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
rem(0.0, 0.0) => NaN
rem(x, y)     => +inf if sign(x) == sign(y) for any x and y not being +inf, -inf, or NaN
rem(x, y)     => -inf if sign(x) != sign(y) for any x and y not being +inf, -inf, or NaN
rem(x, +inf)  => x for any x except +inf, -inf, and NaN
rem(x, -inf)  => x for any x except +inf, -inf, and NaN
rem(+inf, y)  => NaN for any Float y
rem(-inf, y)  => NaN for any Float y
rem(NaN, y)   => NaN for any Float y
rem(x, NaN)   => NaN for any Float x
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.rem(7.2, 2.3), 0.3, epsilon);
```

## Function `pow`
``` motoko no-repl
func pow(x : Float, y : Float) : Float
```

Returns `x` to the power of `y`, `x ** y`.

Note: Numerical errors may occur, see comment above.

Special cases:
```
pow(+inf, y)    => +inf for any y > 0.0 including +inf
pow(+inf, 0.0)  => 1.0
pow(+inf, y)    => 0.0 for any y < 0.0 including -inf
pow(x, +inf)    => +inf if x > 0.0 or x < 0.0
pow(0.0, +inf)  => 0.0
pow(x, -inf)    => 0.0 if x > 0.0 or x < 0.0
pow(0.0, -inf)  => +inf
pow(x, y)       => NaN if x < 0.0 and y is a non-integral Float
pow(-inf, y)    => +inf if y > 0.0 and y is a non-integral or an even integral Float
pow(-inf, y)    => -inf if y > 0.0 and y is an odd integral Float
pow(-inf, 0.0)  => 1.0
pow(-inf, y)    => 0.0 if y < 0.0
pow(-inf, +inf) => +inf
pow(-inf, -inf) => 1.0
pow(NaN, y)     => NaN if y != 0.0
pow(NaN, 0.0)   => 1.0
pow(x, NaN)     => NaN for any Float x
```

Example:
```motoko include=import
let epsilon = 1e-6;
assert Float.equal(Float.pow(2.5, 2.0), 6.25, epsilon);
```
