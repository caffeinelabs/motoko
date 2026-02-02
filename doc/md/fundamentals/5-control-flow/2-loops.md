---
sidebar_position: 2
---

# Loops

<!-- TODO: consolidate with basic-control-flow - some redundant material here -->

In Motoko, loops provide flexible control over repetition, such as iterating over collections, looping while some condition holds, or just looping until an explicit exit from the loop.

Motoko supports different types of loops:

- `loop` loops: Repeat until explicitly exited.

- `loop-while` loops: Repeat until condition is false (tests after each iteration).

- `for` loops: Iteration over collections.

- `while` loops: Repeat while condition is true (tests before each iteration).

## Unconditional loops

An unconditional loop runs indefinitely until it is explicitly stopped. Unlike `while` or `for` loops, which rely on a condition to determine when to exit, unconditional loops continue executing without any predefined exit condition. They are useful in scenarios where the program waits for an external event or depends on a break condition defined within the loop body.

Motoko uses the `loop` keyword to define an infinite loop. To exit such a loop, you can use a `break` statement that will exit the innermost loop, or `break <label>` to exit the labeled loop.

```motoko no-repl
import Debug "mo:core/Debug";

var count : Nat = 0;
loop {
  if (count > 5) break;
  Debug.print(debug_show(count));
  count += 1;
}
```

You can also use a labeled loop when you need to break from a specific loop in nested loops:

```motoko no-repl
import Debug "mo:core/Debug";

var count : Nat = 0;
label countLoop loop {
  if (count > 5) break countLoop;
  Debug.print(debug_show(count));
  count += 1;
}
```

## `loop-while`

A `loop-while` executes the loop body at least once, then repeats as long as the condition remains true.

``` motoko no-repl
import Debug "mo:core/Debug";

var count = 0;
loop {
  Debug.print(debug_show(count));
  count += 1;
} while (count < 5);
```

:::note
Even if the condition starts as false, the body runs at least once.
:::

## `for` loop

A `for` loop in Motoko is used to iterate over the elements of a collection using the following structure:

```motoko no-repl
for (pattern in iterator) {
  // Code to run for each item
}
```

A `for` loop must satisfy these conditions:

- The iterator must provide a `next()` function that returns either a value (`?T`) or `null` when no items remain.

- The pattern must match the type of each item returned by the iterator.

- The pattern cannot fail to match, e.g. by  matching on a particular number.

The `for` loop's iterator is evaluated once at the start. Each time through the loop, `next()` is called:

- If it returns a value, it is matched to the pattern, and the loop body runs with the identifiers declared by the pattern.

- If it returns `null`, the loop stops.

If evaluating the iterator causes a trap (error), the loop stops immediately.

```motoko no-repl
import Debug "mo:core/Debug";

let numbers = [0, 1, 2, 3, 4];

for (num in numbers.vals()) {
  Debug.print(debug_show(num));
};
```

The pattern can also match on values, for example:

```motoko no-repl
import Debug "mo:core/Debug";

let pairs = [(1, 2), (3, 4)];

for ((fst, snd) in pairs.vals()) {
  Debug.print(debug_show(fst + snd));
};
```

## `while` loop

A `while` loop in Motoko repeatedly executes a block of code as long as a given condition is `true`.

```motoko no-repl
while (condition) {
  // Code to run while the condition is true
}
```

A `while` loops must satisfy these constraints:

- The condition must have a type  [`Bool`](../../core/Bool.md) producing `true` or `false`.

- The loop body must have type `()` such that it doesn't return a meaningful value.

The `while` loop first evaluates the condition:

- If it evaluates to `false`, the loop ends and does nothing.

- If it evaluates to `true`, the body runs and the loop repeats.

Once the condition becomes false, the loop stops, and the final result is `()`.

```motoko no-repl
import Debug "mo:core/Debug";

var count = 3;
while (count > 0) {
  Debug.print("Counting down...");
  count -= 1;
};
```

## Continuing loops

You can skip to the next iteration of a loop using the `continue` expression.

```motoko no-repl
import Debug "mo:core/Debug";

var count = 8;
while (count > 0) {
  count -= 1;
  if (count % 2 == 0) continue;
  Debug.print("Counting down...");
};
```

If a loop is labeled with a label `l` then you can continue to the next iteration of the loop labeled `l` using the expression `continue l`.
This is useful when you have nested loops and need to continue a specific outer loop.

```motoko no-repl
import Debug "mo:core/Debug";

var count = 8;
label l while (count > 0) {
  count -= 1;
  if (count % 2 == 0) continue l;
  Debug.print("Counting down...");
};
```

## Abandoning loops

While each type of loop has its own exit conditions and continues execution afterward, you can always exit a loop early using `return`, `throw`, `break`, or `!` (a `null` break) when supported. These provide more control over when and how the loop ends.

