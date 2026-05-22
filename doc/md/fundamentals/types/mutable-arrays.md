---
title: "Mutable arrays"
description: "Mutable arrays allow direct modification of elements, making them suitable for scenarios where data needs to be updated frequently."
sidebar:
  order: 9
---

Mutable arrays allow direct modification of elements, making them suitable for scenarios where data needs to be updated frequently. Unlike [immutable arrays](./immutable-arrays.md), which require creating a new array to reflect changes, mutable arrays support in place modifications, improving performance in some cases.

## Creating a mutable array

Mutable array types are written with square brackets `[var T]`. The `var` keyword indicates mutability. The type of the array element is specified within the square brackets, e.g., `[var Nat]` describes an mutable array of natural numbers.

A mutable array is created using a mutable array expression:

 ```motoko no-repl
 [var 1, 2, 3, 4, 5];
 ```

:::note
Its type is inferred to be `[var Nat]`.
:::

If you want to update the array with negative elements, use a type annotation:

 ```motoko no-repl
 [var 1, 2, 3, 4, 5] : [var Int]
 ```

A named array can be declared using either `let` or `var`:

```motoko no-repl
let digits = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];
```

The function `Array.tabulateVar(size, f)` creates a mutable array of `size` elements, where each element at index `i` is initialized with the value `f(i)`.

Example:

```motoko no-repl
import Nat "mo:core/Nat";
import Array "mo:core/Array";

let digits = Array.tabulateVar<Text>(10, Nat.toText);
```

Constructs the array:

```motoko no-repl
[var "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
```

Each element is mutable and can be updated later.

To initialize a large array where every element starts with the same value, use `Array.init(size, value)`:

```motoko no-repl
import Array "mo:core/Array";

let optArr = Array.repeat<?Int>(null, 10);
```

This produces the array:

```motoko no-repl
[var null, null, null, null, null, null, null, null, null, null]
```

Such arrays are useful when the exact contents will be updated later.

## When to use mutable arrays

Mutable arrays are beneficial when:

- Frequent modifications are required without the overhead of creating a new array.
- Dynamic algorithms need in place operations, such as sorting or shuffling.

## Immutable arrays vs mutable arrays

| Feature          | Immutable arrays                          | Mutable arrays                        |
|-----------------|------------------------------------------|---------------------------------------|
| Syntax          | `[T]`                                    | `[var T]`                            |
| Mutability      | Cannot be updated after creation.        | Can be updated in place.              |
| Modification    | Requires creating a new array.          | Can modify elements directly.         |
| Growth          | Not designed to grow.                   | Not designed to grow.                 |
| Shareability    | Can be shared across functions and actors. | Not sharable.       |
| Conversion      | Can be converted to mutable with `Array.thaw`. | Can be converted to immutable with `Array.freeze`. |
| Use case        |  Tabular fixed, data       |  Iterative algorithms |

:::caution

Unlike some other programming languages that support resizable arrays, Motoko's arrays (both mutable and immutable) are fixed-size. Motoko arrays cannot shrink or grow in length, and operations like `Array.concat` always construct new arrays.
For dynamically-sized, array-like data structures, consider using modules in `core` (e.g. `List`) or other `mops` packages (e.g. [`vector`](https://mops.one/vector)).

:::

## Defining a mutable array

Mutable arrays use the `var` keyword inside the square brackets `[var T]`. The type of the array is also specified within the square brackets, e.g., `[var Nat]` declares a mutable array of natural numbers. In place element modification is supported in mutable arrays.

```motoko no-repl
let mutableArray : [var Nat] = [var 1, 2, 3, 4, 5];

mutableArray[0] := 10;  // Updates the first element to 10

mutableArray;
```

## Accessing and modifying elements

Mutable array elements can be read and modified using indexed access. Attempting to access an index that does not exist will result in a [trap](../basic-syntax/traps.md).

```motoko no-repl
let numbers : [var Nat] = [var 10, 20, 30];

numbers[0] := 100;  // updating first element

debug_show(numbers[0]);  // 100
```

The size of an array `a` is available as `a.size()`, a `Nat`.  Array elements are zero-indexed, allowing indices `0` up to `a.size() - 1`.

Attempting to access an array's index that does not exist will cause a [trap](../basic-syntax/traps.md).

```motoko no-repl
let numbers : [var Nat] = [var 10, 20, 30];
let first : Nat = numbers[0];  // 10
let second : Nat = numbers[1]; // 20
Debug.print(debug_show(first));
Debug.print(debug_show(second));
```

## Iterating through an array

There are two primary ways to iterate through the elements in an array in Motoko:

1. Using `array.values()`, which provides an iterator.
2. Using a `for` loop that runs from `0` to `array.size() - 1`, as arrays are zero-based.

Both methods achieve the same result, but `array.values()` is often preferred for its readability and simplicity.

### Using `array.values()` and `array.keys()`

The `array.values()` function returns an iterator that is used to iterate over the array's elements without manually managing indices.

The `array.keys()` function returns an iterator that is used to iterate over the array's valid indices (in increasing order).

### Using a `for` loop

A `for` loop can also be used to iterate over an array by accessing elements via their index.

```motoko no-repl
let arr = [var "a", "b", "c"];
for (i in arr.keys()) {
    Debug.print(arr[i]);
}
```

## Converting a mutable array to an immutable array

You can convert a mutable array into an immutable array using `Array.freeze`, ensuring that the contents cannot be modified after conversion. Since mutable arrays are not [sharable](./shared-types.md), freezing them is useful when passing data across [functions](./function-types.md) or [actors](../actors/actors-async.md) to ensure immutability.

```motoko no-repl
import Array "mo:core/Array";

let varArray : [var Nat] = [var 1, 2, 3];

let array : [Nat] = Array.fromVarArray(varArray);
```

## Nested mutable arrays example: Tic-tac-toe

To demonstrate nested mutable arrays, consider the following.

A Tic-tac-toe board is a `3x3` grid that requires updates as players take turns. Since elements must be modified, a nested mutable array is the ideal structure.

`VarArray.tabulate` is used to create a mutable board initialized with `"_"` (empty space).

```motoko no-repl
import VarArray "mo:core/VarArray";
import Debug "mo:core/Debug";

persistent actor TicTacToe {
func createTicTacToeBoard() : [var [var Text]] {
    let size : Nat = 3;

    // Initialize a 3x3 board with empty spaces
    VarArray.tabulate<[var Text]>(
      size,
      func(_ : Nat) : [var Text] {
        VarArray.tabulate<Text>(size, func(_ : Nat) : Text {"_"}) // Fill with "_"
      }
    )
  };

  // Create a mutable Tic-tac-toe board
  let board : [var [var Text]] = createTicTacToeBoard();

  // Function to make a move
  func makeMove(row : Nat, col : Nat, player : Text) {
    if (board[row][col] == "_") {
      board[row][col] := player
    } else {
      Debug.print("Invalid move! The spot is already taken.")
    }
  };

  // Function to print the board
  func printBoard() {
    for (row in board.vals()) {
      let rowText = Array.foldLeft<Text, Text>(Array.freeze<Text>(row), "", func(acc, cell) = acc # cell # " ");
      Debug.print(rowText)
    }
  };

  // Example moves
  makeMove(0, 0, "X");
  makeMove(1, 1, "O");
  makeMove(2, 2, "X");

  printBoard();
};
```

Since both the outer and inner arrays are mutable, players can update the board in place. The array must be frozen before `foldLeft()` can be applied to the rows as `foldleft()` expects an immutable array as an argument.

```md
X _ _
_ O _
_ _ X
```

## Subtyping

For safety reason, mutable arrays do not support subtyping. This means that `[var T]` is a subtype of `[var U]` only when the types `T` and `U` are, in fact, equal. It is not enough for `T` to be a subtype of `U`, as some users might expect.

To see why, suppose the following was allowed: `[var Nat] <: [var Int]` (since `Nat <: Int`).

Then, consider the following code:

```motoko no-repl
let ns :  [var Nat] = [var 0];
let is  :  [var Int] = ns; // only allowed if [var Nat] <: [var Int]
is[0] := -1; // [var Nat] is not a subtype of [var Int] — even though Nat <: Int.
ns[0] // -1
```

Here, `ns` starts out as an array of non-negative `Nat`s, storing `0` in its only element.
Declaring `is`, of type to `[var Int]` creates an alias of `ns`, but at the super type `[var Int]`. Now since `is` can store `Int`s, we can assign `-1` to `is[0]`, and then read `-1` from `ns[0]`, breaking the promise that `ns` is an array of non-negative numbers.

## Resources

- [`Array`](https://mops.one/core/docs/Array)

