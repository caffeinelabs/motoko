---
title: "Tuples"
description: "A tuple is a fixed-size, ordered collection of values, where each element can have a different type."
sidebar:
  order: 4
---

A tuple is a fixed-size, ordered collection of values, where each element can have a different type. Tuples provide a way to group values by position, without the overhead of defining a record of named fields.

A tuple is grouped together in parentheses (`value1`, `value2`, `value3`). The type of a tuple is determined by types of its elements, such as ([`Text`](https://mops.one/core/docs/Text), [`Nat`](https://mops.one/core/docs/Nat), [`Bool`](https://mops.one/core/docs/Bool)). The values inside a tuple are evaluated in order from left to right. Tuples are immutable and their components cannot be modified after creation (unlike record fields, which can be declared mutable).

A tuple with zero elements is called the **unit value**, written as `()`. Since it carries no other data, it represents a trivial or token value. It's type is the **unit type**, also written `()`.

Unit values are typically used as placeholder arguments or return values for functions that don't require input or don't return meaningful results. However, such functions may still produce side effects, such as printing to the console or modifying state.

## Defining a tuple

```motoko no-repl
let ghost = ("Motoko", 25);
```

The tuple's type is automatically inferred as `(Text, Nat)`, since `"Motoko"` is of type [`Text`](https://mops.one/core/docs/Text) and `25` is of type [`Nat`](https://mops.one/core/docs/Nat).

```motoko no-repl
let ghost : (Text, Nat) = ("Motoko", 25);
```

:::note[Support for one length tuples]
Motoko **does not** support length-one tuples. This is in contrast to languages such as Python, where a trailing comma differentiates a single-element tuple from a simple parenthesized value. In Motoko, `(x)` is always just `x`.
:::

## Accessing elements

Elements are accessed using `.n` where `n` is the index (0-based indexing).

```motoko no-repl
// Ghost is a tuple of length 2
let ghost : (firstName : Text, age : Nat) = ("Motoko", 25);
let first = ghost.0; // "Motoko"
let second = ghost.1; // 25
```

## Tuples as function return values

Tuples are useful for returning multiple values from a function without requiring a separate data structure.

```motoko no-repl
func getUserInfo() : (Text, Nat) {
    ("Ghost", 30);
};

getUserInfo();
```

## Using tuples in collections

Tuples can be stored in arrays or other data structures. Tuples can be constructed with named types, improving readability. By naming the types in the tuple in the collection, the intent of each component is clarified, reducing ambiguity.

```motoko no-repl
let users : [(Text, Nat)] = [("Motoko", 25), ("Ghost", 30)];
```

This structure efficiently represents a collection of key-value pairs without requiring a dedicated [record](./records.md) type.

## Pattern matching on tuples

In addition to dot notation, tuples can be decomposed using tuple patterns. When combined with `let` or `switch`, this allows you to access the components of a tuple through simple pattern matching.

```motoko no-repl
let users : [(Text, Nat)] = [("Motoko", 25), ("Ghost", 30)];

let (firstUserName, _) = users[0] // "Motoko"
```

The array `users` contains tuples, where each tuple represents a user with a [`Text`](https://mops.one/core/docs/Text) name and a [`Nat`](https://mops.one/core/docs/Nat) age.
Accessing `users[0]` retrieves the first tuple in the array.
A `let` pattern can then be used to extract just the name, ignoring the age by using the wildcard pattern (`_`).

## Nesting tuples

Tuples can be used to represent coordinate pairs, allowing for structured calculations such as finding the gradient of a line. The gradient (or slope) between two points `$(x_1, y_1)$` and `$(x_2, y_2)$` is calculated using the formula:

$$
m = \frac{y_2 - y_1}{x_2 - x_1}
$$

Using nested tuples, this can be implemented in Motoko as follows:

```motoko no-repl
// Point is a tuple of coordinate floats
type Point = (Float, Float);
// Line is a tuple of points, that is, a nested tuple
type Line = (Point, Point);
func calculateGradient(line : Line) : ?Float {
  let ((x1, y1), (x2, y2)) = line;
  if (x1 == x2) {
    null
    // Gradient is undefined for a vertical line
  } else {
    ?((y2 - y1) / (x2 - x1)) // Wraps the result as an option
  }
};
let line : Line = ((2.0, 3.0), (5.0, 7.0));
// Show the gradient when run
calculateGradient(line);
```

