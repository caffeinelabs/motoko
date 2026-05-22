---
title: "Conditionals"
description: "Conditionals in Motoko come in two forms: if-expressions and if-statements."
sidebar:
  order: 3
---

Conditionals in Motoko come in two forms: **if-expressions** and **if-statements**.

## `if-else`

An `if-else` expression, `if <c> <b1> else <b2>`, has three parts:

1. A condition `<c>` that evaluates to a boolean value.
2. Two branches `<b1>` and `<b2>`. These branches can be simple expressions or blocks `{ ... }`.

If the condition is `true`, the first branch `<b1>` is evaluated to produce the result of the `if-else`.
If the conditions is `false`, the second branch  `<b2>`  is evaluated to produce the result of `if-else`.
Only one of the branches is evaluated.

The type of the`if-else` is the type of `<b1>` and `<b2>` if they have the same type; otherwise, it is a more general type that is a supertype of their types.

For example, you might use an `if-else` to choose a label based on a value.

```motoko no-repl
let x : Int = 1;

let identity : Text =
  if (x == 1) {
    "x is 1"
  } else {
    "x is not 1"
  }; // Produces a value
```

The result of the `if-else` is assigned to `identity`. Here, both branches have the same type ([`Text`](https://mops.one/core/docs/Text) in this case) as does the entire `if-else`.

```motoko no-repl
let n : Nat = 0;
let parity = if (n % 2 == 0) #even else #odd;
```
Here, the first branch has type `{#even}` and the second branch has type `{#odd}`. These types are different but they have a common supertype `{#even; #odd}`. The type of the `if-else` is then `{#even; #odd}`.

Motoko will infer the common supertype for you, choosing the most specific one possible. If the types are inconsistent and only have the useless common supertype `Any`, Motoko will issue a warning:

```motoko no-repl
let n : Nat = 0;
let oops = if (n % 2 == 0) #even else 0;
```

## `if`-expression

An `if`-expression takes the form `if <c> <b1>` and is like an `if-else` but omits the else and second branch `else <b2>`.

An `if`-expression is used purely for its side effects to conditionally evaluate a single branch when the condition is true, and do nothing otherwise. It returns the trivial value `()`, and its type is `()`.

`if`-expressions are best suited for situations where you need to perform conditional actions, such as logging or modifying state based on certain conditions.

Since `if`-expressions have type `()`, they can be used as declaration expressions.

```motoko no-repl
let x : Int = 1;

if (x == 1) {
    Debug.print("x is 1"); // Prints and returns ()
};
```


## Nesting `if-else` expressions

`if-else` expressions can be nested and associate to the right. This ensures the following code works as intended and the second `else` belongs to the second, nested `if`.

```motoko no-repl
var age = 21;

if (age < 18) {
  "You are a minor."
} else if (age >= 18 and age < 65) {
  "You are an adult."
} else {
  "You are a senior citizen."
};
```


