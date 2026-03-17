---
sidebar_position: 1
---

# Primitive types

Motoko provides several primitive types that form the foundation of all computations. These include numeric types, characters and text, booleans, and floating-point numbers.

The primitive types are supported by a large set of familiar built-in operators such as `+`, `-` and so on.

More esoteric functions, not supported by dedicated operators, can be found in the corresponding libraries.

For example, the library function `Int.toText: Int -> Text`, declared in core package `Int`, returns the textual representation of its argument.

```motoko name=int
import Int "mo:core/Int";
Int.toText(0); // returns "0"
```

## Numeric types

Motoko supports both signed integers and unsigned naturals. Signed numbers can represent all numbers, positive and negative, while unsigned integers can only represent 0 and positive numbers. Natural numbers are unsigned integers.

- Signed integers: [`Int`](../../core/Int.md), [`Int8`](../../core/Int8.md), [`Int16`](../../core/Int16.md), [`Int32`](../../core/Int32.md), [`Int64`](../../core/Int64.md)
- Unsigned naturals: [`Nat`](../../core/Nat.md), [`Nat8`](../../core/Nat8.md), [`Nat16`](../../core/Nat16.md), [`Nat32`](../../core/Nat32.md), [`Nat64`](../../core/Nat64.md)

The [`Int`](../../core/Int.md) and [`Nat`](../../core/Nat.md) types prevent overflow and underflow since they can represent values of arbitrary size. Of course, subtraction on a [`Nat`](../../core/Nat.md) can still result in underflow if the result would be negative.

In Motoko, [`Nat`](../../core/Nat.md) is a subtype of [`Int`](../../core/Int.md), since the set of non-negative integers is a subset of all integers.

This means that every expression of type [`Nat`](../../core/Nat.md) can implicitly serve as an [`Int`](../../core/Int.md) without any need for conversion. The opposite is not true.

An [`Int`](../../core/Int.md) cannot be directly assigned to a [`Nat`](../../core/Nat.md) since it may be a negative number and the [`Nat`](../../core/Nat.md) type only contains non-negative numbers.

```motoko
let x : Int = -5;
let y : Nat = x; // Error
```

Passing an [`Int`](../../core/Int.md) as a [`Nat`](../../core/Nat.md) equires an explicit conversion, such as taking the absolute value or applying another conversion function.

```motoko no-repl
let x : Int = -5;
let y : Nat = Int.abs(x); // Allowed, y = 5
```

Fixed-size numeric types ([`Int8`](../../core/Int8.md), [`Nat32`](../../core/Nat32.md), etc.) support additional operations, including bitwise shifts.

```motoko
let x : Nat32 = 0xA; // 10 in hexadecimal
let y = x << 2; // 0x28 (40 in decimal)
```

## `Char` and `Text`

`Char` represents a single Unicode scalar value, while [`Text`](../../core/Text.md) represents a sequence of characters.

```motoko
import Char "mo:core/Char";
import Text  "mo:core/Text";

let letter : Char = 'A';

let codePoint = Char.toNat32(letter); // 65

let word : Text = "Motoko";
let uppercase = Text.toUpper(word); // "MOTOKO"

let modified = Text.replace("hello world", #text "world", "Motoko"); // "hello Motoko"
let words = Text.split("apple,banana,cherry", #char ','); // apple -> banana -> cherry
```

## Bool

The [`Bool`](../../core/Bool.md) type represents boolean values, `true` or `false`, and supports logical operations.

The logical operators `and` and `or` will only evaluate their second operand if necessary.

```motoko
let flag : Bool = true or false; // true
let opposite = not flag; // false

let isEqual =  true == false ; // false
```

## Float

[`Float`](../../core/Float.md) is a 64-bit floating-point type that provides mathematical operations.

```motoko
import Float "mo:core/Float";
let pi = Float.pi;
let radius : Float = 2.5;
let area = Float.pow(radius, 2) * pi; // Area of a circle

let rounded = Float.floor(4.8); // 4.0
let trigValue = Float.sin(Float.pi / 2); // 1.0
```

## Float32

`Float32` is a 32-bit (single-precision) IEEE 754 floating-point type. It uses half the
memory of [`Float`](../../core/Float.md) and maps directly to the `float32` Candid type,
making it suitable for compact storage or interoperability with services that use
single-precision values.

Values are written as float literals with a `Float32` type ascription, or produced by
arithmetic on `Float32` operands, or by explicit conversion from `Float`:

```motoko
let a : Float32 = 3.14;              // literal rounded to single precision at compile time
let b : Float32 = a * 2.0;           // arithmetic stays in Float32
let c : Float32 = floatToFloat32 3.14;   // explicit conversion from Float
let back : Float = float32ToFloat a;
```

Arithmetic (`+`, `-`, `*`, `/`, `**`) and comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`)
work directly on `Float32` values without going through `Float`.

:::note
`Float32` has approximately 7 significant decimal digits of precision, compared to ~15 for
[`Float`](../../core/Float.md). Use `Float` when precision matters; use `Float32` when
memory compactness or Candid `float32` interoperability is the priority.
:::

## Resources

- [`Int`](../../core/Int.md)
- [`Nat`](../../core/Nat.md)
- [`Bool`](../../core/Bool.md)
- [`Blob`](../../core/Blob.md)
- [`Char`](../../core/Char.md)
- [`Text`](../../core/Text.md)
- [`Float`](../../core/Float.md)
- `Float32`

