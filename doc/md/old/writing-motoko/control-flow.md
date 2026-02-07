---
sidebar_position: 8
---

# Control flow



There are two key categories of control flow:

-   **Declarative** : The structure of some value guides both control and the selection of the next expression to evaluate, such as in `if` and `switch` expressions.

-   **Imperative** : Control changes abruptly according to a programmer’s command, abandoning regular control flow. Examples are `break` and `continue`, but also `return` and `throw`.

Imperative control flow often goes hand-in-hand with state changes and other side effects, such as error handling and input/output.

## Early `return` from `func`

Typically, the result of a function is the value of its body. Sometimes, during evaluation of the body, the result is available before the end of evaluation. In such situations the `return <exp>` construct can be used to abandon the rest of the computation and immediately exit the function with a result. Similarly, where permitted, `throw` may be used to abandon a computation with an error.

When a function has unit result type, the shorthand `return` may be used instead of the equivalent `return ()`.

## Loops and labels

Motoko provides several kinds of repetition constructs, including:

-   `for` expressions for iterating over members of structured data.

-   `loop` expressions for programmatic repetition, optionally with termination condition.

-   `while` loops for programmatic repetition with entry condition.

You can use `break` and `continue` (without labels) in loops to exit or continue the innermost loop.
Any of these loops can also be prefixed with a `label <name>` qualifier to give the loop a symbolic name. Labeled loops are useful for imperatively changing control flow in nested loops or when you need to target a specific loop, such as:

-   Re-entering the loop with `continue <name>` (or `continue` without a name for the innermost loop).

-   Exiting the loop altogether with `break <name>` (or `break` without a name for the innermost loop).

In the following example, the `for` expression loops over characters of some text and abandons iteration as soon as an exclamation sign is encountered.

``` motoko
import Debug "mo:core/Debug";
label letters for (c in "ran!!dom".chars()) {
  Debug.print(debug_show(c));
  if (c == '!') { break letters };
  // ...
}
```

### Labeled expressions

There are two other facets to `label`​s that are less mainstream, but come in handy in certain situations:

-   `label`​s can be typed.

-   Any expression, not just loops, can be named by prefixing it with a label. `break` allows one to short-circuit the expression’s evaluation by providing an immediate value for its result. This is similar to exiting a function early using `return`, but without the overhead of declaring and calling a function.

The syntax for type-annotated labels is `label <name> : <type> <expr>`, signifying that any expression can be exited using a `break <name> <alt-expr>` construct that returns the value of `<alt-expr>` as the value of `<expr>`, short-circuiting evaluation of `<expr>`.

Judicious use of these constructs allows the programmer to focus on the primary program logic and handle exceptional case via `break`.

``` motoko
import Text "mo:core/Text";
import Iter "mo:core/Iter";

type Host = Text;
let formInput = "us@dfn";

let address = label exit : ?(Text, Host) {
  let splitted = Text.split(formInput, #char '@');
  let array = Iter.toArray<Text>(splitted);
  if (array.size() != 2) { break exit(null) };
  let account = array[0];
  let host = array[1];
  // if (not (parseHost(host))) { break exit(null) };
  ?(account, host)
}
```

Labeled common expressions don’t allow `continue`. In terms of typing, both `<expr>` and `<alt-expr>`​'s types must conform with the label’s declared `<type>`. If a label is only given a `<name>`, then its `<type>` defaults to unit (`()`). Similarly a `break` without an `<alt-expr>` is shorthand for the value unit (`()`).

## Option blocks and null breaks

Motoko lets you opt in to `null` values, tracking possible occurrences of `null` values using option types of the form `?T`. This is both to encourage you to avoid using `null` values when possible, and to consider the possibility of `null` values when necessary. Motoko simplifies the handling of option types with some dedicated syntax: option blocks and null breaks.

The option block, `do ? <block>`, produces a value of type `?T`, when block `<block>` has type `T` and, importantly, introduces the possibility of a break from `<block>`. Within a `do ? <block>`, the null break `<exp> !`, tests whether the result of the expression, `<exp>`, of unrelated option type, `?U`, is `null`. If the result `<exp>` is `null`, control immediately exits the `do ? <block>` with value `null`. Otherwise, the result of `<exp>` must be an option value `?v`, and evaluation of `<exp> !` proceeds with its contents, `v` of type `U`.

The following example defines a simple function that evaluates expressions built from natural numbers, division and a zero test, encoded as a variant type:

<!--
TODO: make interactive
-->

``` motoko file=../../examples/option-block.mo
```

To guard against division by `0` without trapping, the `eval` function returns an option result, using `null` to indicate failure.

Each recursive call is checked for `null` using `!`, immediately exiting the outer `do ? block`, and then the function itself, when a result is `null`.

## Repetition with `loop`

The simplest way to indefinitely repeat a sequence of imperative expressions is by using a `loop` construct:

``` motoko no-repl
loop { <expr1>; <expr2>; ... }
```

The loop can only be abandoned with a `return` or `break` construct.

A re-entry condition can be affixed to allow a conditional repetition of the loop with `loop <body> while <cond>`.

The body of such a loop is always executed at least once.

## `while` loops with precondition

Sometimes an entry condition is needed to guard each iteration of a loop. For this kind of repetition use the `while <cond> <body>` form of loop:

``` motoko no-repl
while (earned < need) { earned += earn() };
```

Unlike a `loop`, the body of a `while` loop may never be executed.

## `for` loops for iteration

An iteration over elements of some homogeneous collection can be performed using a `for` loop. The values are drawn from an iterator and bound to the loop pattern in turn.

``` motoko
let carsInStock = [
  ("Buick", 2020, 23.000),
  ("Toyota", 2019, 17.500),
  ("Audi", 2020, 34.900)
];
var inventory : { var value : Float } = { var value = 0.0 };
for ((model, year, price) in carsInStock.values()) {
  inventory.value += price;
};
inventory
```

## Using `range` with a `for` loop

The `range` function produces an iterator of type `Iter<Nat>` with the given lower and upper bound, inclusive.

The following loop example prints the numbers `0` through `10` over its eleven iterations:

``` motoko
import Nat "mo:core/Nat";
import Debug "mo:core/Debug";
var i = 0;
for (j in Nat.range(0, 10)) {
  Debug.print(debug_show(j));
  assert(i == j);
  i += 1;
};
assert(i == 11);
```

More generally, the function `range` is a `class` that constructs iterators over sequences of natural numbers. Each such iterator has type `Iter<Nat>`.

As a constructor function, `range` has a function type:

``` motoko no-repl
(lower : Nat, upper : Int) -> Iter<Nat>
```

`Iter<Nat>` is an iterator object type with a `next` method that produces optional elements, each of type `?Nat`:

``` motoko no-repl
type Iter<A> = {next : () -> ?A};
```

For each invocation, `next` returns an optional element of type `?Nat`.

The value `null` indicates that the iteration sequence has terminated.

Until reaching `null`, each non-`null` value, of the form `?n` for some number `n`, contains the next successive element in the iteration sequence.

## Using `revRange`

The function `revRange` is a `class` that constructs iterators, each of type `Iter<Int>`. As a constructor function, it has a function type:

``` motoko no-repl
(upper : Int, lower : Int) -> Iter<Int>
```

Unlike `range`, the `revRange` function descends in its iteration sequence, from an initial upper bound to a final lower bound.

## Using iterators of specific data structures

Many built-in data structures come with pre-defined iterators. Below table lists them

| Type      | Name                  | Iterator | Elements                  | Element type |
|-----------|-----------------------|----------|---------------------------|--------------|
| `[T]`     | Array of `T`​s         | `vals`   | The array’s members       | `T`          |
| `[T]`     | Array of `T`​s         | `keys`   | The array’s valid indices | [`Nat`](../core/Nat.md)        |
| `[var T]` | Mutable array of `T`​s | `vals`   | The array’s members       | `T`          |
| `[var T]` | Mutable array of `T`​s | `keys`   | The array’s valid indices | [`Nat`](../core/Nat.md)        |
| [`Text`](../core/Text.md)    | Text                  | `chars`  | The text’s characters     | `Char`       |
| [`Blob`](../core/Blob.md)    | Blob                  | `vals`   | The blob’s bytes          | [`Nat8`](../core/Nat8.md)       |


User-defined data structures can define their own iterators. As long they conform with the `Iter<A>` type for some element type `A`, these behave like the built-in ones and can be consumed with ordinary `for`-loops.

<img src="https://github.com/user-attachments/assets/844ca364-4d71-42b3-aaec-4a6c3509ee2e" alt="Logo" width="150" height="150" />