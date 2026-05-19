---
title: "Primitive types"
description: "Motoko provides several primitive types that form the foundation of all computations."
sidebar:
  order: 1
---

Motoko provides several primitive types that form the foundation of all computations. These include numeric types, characters and text, booleans, and floating-point numbers.

The primitive types are supported by a large set of familiar built-in operators such as `+`, `-` and so on.

More esoteric functions, not supported by dedicated operators, can be found in the corresponding libraries.

For example, the library function `Int.toText: Int -> Text`, declared in core package `Int`, returns the textual representation of its argument.

```motoko no-repl
import Int "mo:core/Int";
Int.toText(0); // returns "0"
```

## Numeric types

Motoko supports both signed integers and unsigned naturals. Signed numbers can represent all numbers, positive and negative, while unsigned integers can only represent 0 and positive numbers. Natural numbers are unsigned integers.

- Signed integers: [`Int`](https://mops.one/core/docs/Int), [`Int8`](https://mops.one/core/docs/Int8), [`Int16`](https://mops.one/core/docs/Int16), [`Int32`](https://mops.one/core/docs/Int32), [`Int64`](https://mops.one/core/docs/Int64)
- Unsigned naturals: [`Nat`](https://mops.one/core/docs/Nat), [`Nat8`](https://mops.one/core/docs/Nat8), [`Nat16`](https://mops.one/core/docs/Nat16), [`Nat32`](https://mops.one/core/docs/Nat32), [`Nat64`](https://mops.one/core/docs/Nat64)

The [`Int`](https://mops.one/core/docs/Int) and [`Nat`](https://mops.one/core/docs/Nat) types prevent overflow and underflow since they can represent values of arbitrary size. Of course, subtraction on a [`Nat`](https://mops.one/core/docs/Nat) can still result in underflow if the result would be negative.

In Motoko, [`Nat`](https://mops.one/core/docs/Nat) is a subtype of [`Int`](https://mops.one/core/docs/Int), since the set of non-negative integers is a subset of all integers.

This means that every expression of type [`Nat`](https://mops.one/core/docs/Nat) can implicitly serve as an [`Int`](https://mops.one/core/docs/Int) without any need for conversion. The opposite is not true.

An [`Int`](https://mops.one/core/docs/Int) cannot be directly assigned to a [`Nat`](https://mops.one/core/docs/Nat) since it may be a negative number and the [`Nat`](https://mops.one/core/docs/Nat) type only contains non-negative numbers.

```motoko no-repl
let x : Int = -5;
let y : Nat = x; // Error
```

Passing an [`Int`](https://mops.one/core/docs/Int) as a [`Nat`](https://mops.one/core/docs/Nat) equires an explicit conversion, such as taking the absolute value or applying another conversion function.

```motoko no-repl
let x : Int = -5;
let y : Nat = Int.abs(x); // Allowed, y = 5
```

Fixed-size numeric types ([`Int8`](https://mops.one/core/docs/Int8), [`Nat32`](https://mops.one/core/docs/Nat32), etc.) support additional operations, including bitwise shifts.

```motoko no-repl
let x : Nat32 = 0xA; // 10 in hexadecimal
let y = x << 2; // 0x28 (40 in decimal)
```

## `Char` and `Text`

`Char` represents a single Unicode scalar value, while [`Text`](https://mops.one/core/docs/Text) represents a sequence of characters.

```motoko no-repl
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

The [`Bool`](https://mops.one/core/docs/Bool) type represents boolean values, `true` or `false`, and supports logical operations.

The logical operators `and` and `or` will only evaluate their second operand if necessary.

```motoko no-repl
let flag : Bool = true or false; // true
let opposite = not flag; // false

let isEqual =  true == false ; // false
```

## Float

[`Float`](https://mops.one/core/docs/Float) is a 64-bit floating-point type that provides mathematical operations.

```motoko no-repl
import Float "mo:core/Float";
let pi = Float.pi;
let radius : Float = 2.5;
let area = Float.pow(radius, 2) * pi; // Area of a circle

let rounded = Float.floor(4.8); // 4.0
let trigValue = Float.sin(Float.pi / 2); // 1.0
```

## Float32

`Float32` is a 32-bit (single-precision) IEEE 754 floating-point type. It uses half the
memory of [`Float`](https://mops.one/core/docs/Float) and maps directly to the `float32` Candid type,
making it suitable for compact storage or interoperability with services that use
single-precision values.

Values are written as float literals with a `Float32` type ascription, or produced by
arithmetic on `Float32` operands, or by explicit conversion from `Float`:

```motoko no-repl
let a : Float32 = 3.14;              // literal rounded to single precision at compile time
let b : Float32 = a * 2.0;           // arithmetic stays in Float32
let c : Float32 = floatToFloat32 3.14;   // explicit conversion from Float
let back : Float = float32ToFloat a;
```

Arithmetic (`+`, `-`, `*`, `/`, `**`) and comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`)
work directly on `Float32` values without going through `Float`.

:::note
`Float32` has approximately 7 significant decimal digits of precision, compared to ~15 for
[`Float`](https://mops.one/core/docs/Float). Use `Float` when precision matters; use `Float32` when
memory compactness or Candid `float32` interoperability is the priority.
:::

## Resources

- [`Int`](https://mops.one/core/docs/Int)
- [`Nat`](https://mops.one/core/docs/Nat)
- [`Bool`](https://mops.one/core/docs/Bool)
- [`Blob`](https://mops.one/core/docs/Blob)
- [`Char`](https://mops.one/core/docs/Char)
- [`Text`](https://mops.one/core/docs/Text)
- [`Float`](https://mops.one/core/docs/Float)
- `Float32`

