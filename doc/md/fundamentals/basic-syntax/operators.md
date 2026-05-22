---
title: "Operators"
description: "Motoko provides various operators for working with numbers, text, and boolean values."
sidebar:
  order: 9
---

Motoko provides various operators for working with numbers, text, and boolean values. They can be categorized as follows:

| **Category**   | **Description**                          | **Examples**  |
|---------------|----------------------------------|----------------------|
| Arithmetic | Math operations on numbers      | `+`, `-`, `*`, `/`, `%`, `**` |
| Bitwise    | Operations on individual bits   | `&`, <code>&#124;</code>, `^`, `<<`, `>>`, `<<>`, `<>>` |
| Text       | Text concatenation              | `#` |
| Logical | Logical/boolean operations       | `not`, `and`, `or` |
| Ordered | Comparing values                  | `==`, `!=`, `<`, `>` |

:::note

Bitwise operators can only be used with bounded types, such as `Int8`, `Nat8`.

:::

## Short-circuit evaluation

In Motoko, the logical operators `and` and `or` use short-circuit evaluation:

* `and` evaluates the second operand **only if** the first is `true`.
* `or` evaluates the second operand **only if** the first is `false`.

This avoids unnecessary computation and potential side effects.

### Short circuit `and`

If the first operand is `false`, the second is not evaluated.

```motoko no-repl
let x = false;

if (x and someOtherExp) {
  Debug.print("Unreachable code executed! something is wrong!"); // This should never be printed.
};
```

### Short circuit `or`

If the first operand is `true`, the second is not evaluated.

```motoko no-repl
let y = true;

if (y or someOtherExp) {
  Debug.print("This will be printed");
};
```

## Unary operators

| Operator | Description |
|----------|------------|
| `-`      | Numeric negation |
| `+`      | Numeric identity |
| `^`      | Bitwise negation |

## Relational operators

Relational operators compare two values and return `true` or `false`.

| Operator | Description | Example|
|----------|------------|-----------|
| `==`     | Equals | `a == b` |
| `!=`     | Not equals | `a != b`|
| `<`      | Less than | `a < b` |
| `>`      | Greater than | `a > b` |
| `<=`     | Less than or equal | `a <= b` |
| `>=`     | Greater than or equal | `a >= b`|

## Numeric binary operators

Binary operators combine two numbers to produce a result.

| Operator | Description |  Example|
|----------|------------|------------|
| `+`      | Addition | `a + b` |
| `-`      | Subtraction | `a - b` |
| `*`      | Multiplication | `a * b` |
| `/`      | Division (integer division) | `a / b` |
| `%`      | Modulus (remainder) | `a % b` |
| `**`     | Exponentiation | `a ** b` |

:::caution

Division (`/`) on integers **truncates** decimals. For floating-point division, use `Float.fromInt()`:

```motoko no-repl
let result = Float.fromInt(10) / Float.fromInt(3);
```

:::

## Bitwise operators

Bitwise operators manipulate numbers **at the binary level**.

| Operator | Description |Example |
|----------|------------|----------|
| `&`      | Bitwise AND |`a & b` |
| <code>&#124;</code> | Bitwise OR | <code>a &#124; b</code> |
| `^`      | Bitwise XOR | `a ^ b` |
| `<<`     | Shift left | `a << b` |
| `>>`     | Shift right (must be preceded by a whitespace) |`a >> b` |
| `<<>`    | Rotate left (circular shift) | `a <<> b` |
| `<>`     | Rotate right (circular shift)| `a <> b` |

:::note

Bitwise operators can only be used with bounded types. eg: `Int8`, `Nat8`.

:::

## Wrapping operators

Bounded integers **trap** on overflow, but **wrapping versions** allow overflow behavior.

| Operator | Description | Example |
|----------|------------|------------|
| `+%`     | Addition with wrap-around | `a +% b` |
| `-%`     | Subtraction with wrap-around | `a -% b` |
| `*%`     | Multiplication with wrap-around | `a *% b` |
| `**%`    | Exponentiation with wrap-around | `a **% b` |

## Text operators

| Operator | Description | Example |
|----------|------------|------------|
| `#`      | Concatenates two [`Text`](https://mops.one/core/docs/Text) values | `a # b` |

## Assignment operators

Assignment operators modify variables in place. Both mutable variables declared with `var` and elements of mutable arrays can be assigned new values.

| Operator | Description |Examples|
|----------|------------|---------|
| `:=`     | Assign a value | `a := b` |
| `+=`     | Add and assign | `a += b` |
| `-=`     | Subtract and assign | `a -= b` |
| `*=`     | Multiply and assign | `a *= b` |
| `/=`     | Divide and assign | `a /= b` |
| `#=`     | Concatenate and assign (for [`Text`](https://mops.one/core/docs/Text)) | `a #= b` |

For example:

```motoko no-repl
var done = false; done := true;

let a = [var 1, 2];
a[0] += a[1];
```

## Operator precedence

Operators follow precedence rules, meaning that, in the absence of explicit parentheses, some operators are evaluated before others.

1. Unary operators (`-`, `!`, `^`)
2. Exponentiation (`**`, `**%`)
3. Multiplication & division (`*`, `/`, `%`, `*%`)
4. Addition & subtraction (`+`, `-`, `+%`, `-%`)
5. Bitwise operators (`&`, `|`, `^`)
6. Comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
7. Assignment operators (`:=`, `+=`, `-=`, etc.)

For example:

```motoko no-repl
let result = 10 + 5 * 2; // result = 20
```

Use parentheses to enforce a different order.

```motoko no-repl
let result = (10 + 5) * 2; // result = 30
```

## Pipes

Pipes (`|>`) chain together function calls in a readable way. Instead of nesting function calls, pipes pass the result of one expression as an argument to the next function. The value of the left side of the pipe is referenced on the right side using an underscore (`_`).

```motoko no-repl
func double(n : Int) : Int { n * 2 };
func increment(n : Int) : Int { n + 1 };

let result = 5 |> double(_) |> increment(_); // (5 * 2) + 1 = 11
```

