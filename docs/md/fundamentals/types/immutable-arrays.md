---
title: "Immutable arrays"
description: "Immutable arrays are fixed-size, read-only data structures that allow efficiently storing elements of the same type."
sidebar:
  order: 8
---

Immutable arrays are fixed-size, read-only data structures that allow efficiently storing elements of the same type. Unlike [mutable arrays](./mutable-arrays.md), they cannot be modified after creation, ensuring data integrity and predictable behavior.

## When to use immutable arrays

- Fixed-size storage suffices: The number of elements is known in advance and will not change.
- Performance is required: Arrays provide efficient random access to elements.
- Mutability is not needed: Immutability ensures that no accidental modifications occur.

Unlike mutable arrays, immutable arrays support subtyping and are shared types that can be sent in messages (provided the element type is shared).

If the number of elements may change, collections like `List` is a better choice, as immutable arrays require creating a new array each time an element is added or removed, which is inefficient.

## Defining an immutable array

Immutable arrays are declared using square brackets `[T]`. The type of the array is specified within the square brackets, e.g., `[Nat]` declares an immutable array of natural numbers.

```motoko no-repl
let arr : [Nat] = [1, 2, 3, 4, 5];
```

## Accessing and modifying elements

The size of an array `a` is available as `a.size()`, a `Nat`.

Array elements are zero-indexed, allowing indices `0` up to `a.size() - 1`.

Attempting to access an array's index that does not exist will cause a [trap](../basic-syntax/traps.md). Attempting to modify an immutable array will result in an error `expected mutable assignment target(M0073)`.

```motoko no-repl
import Debug "mo:core/Debug";

let numbers : [Nat] = [10, 20, 30];

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
import Debug "mo:core/Debug";

let arr : [Nat] = [1, 2, 3, 4, 5];

for (i in arr.keys()) {
  Debug.print(debug_show(arr[i]));
}
```

## Converting an immutable array to a mutable array

You can convert an immutable array into a mutable array using `Array.thaw` which is useful when working with data that needs to be modified in place after initially being immutable.

```motoko no-repl
let immutableArray : [Nat] = [1, 2, 3, 4, 5];

let mutableCopy : [var Nat] = Array.thaw<Nat>(immutableArray);
mutableCopy[0] := 10;
```

## Passing a variable number of arguments

Motoko supports passing collections to a function, ensuring that all arguments are handled as a collection rather than individual parameters.

```motoko no-repl
import Debug "mo:core/Debug"

func printAllStrings(strings : [Text]) {
  for (s in strings.values()) {
    Debug.print(s);
  }
};

printAllStrings(["Hello", "Hola", "Ciao"]);
```

## Comparing arrays

Arrays of  shared types can be compare directly using `==`. Two arrays are considered equal if they have the same length and their corresponding elements are equal:

```motoko no-repl
let arr1 : [Nat] = [1, 2, 3];
let arr2 : [Nat] = [1, 2, 3];
let arr3 : [Nat] = [3, 2, 1];

assert arr1 == arr2;
assert (not (arr1 == arr3));
```

More generally, including for arrays of non-shared types, the `Array.equal<T>(a1, a2, eq)` function can be used to check whether two arrays are equal. It takes an additional function `eq` for comparing the elements.

```motoko no-repl
import Array "mo:core/Array";
import Nat "mo:core/Nat";

let arr1 : [Nat] = [1, 2, 3];
let arr2 : [Nat] = [1, 2, 3];
let arr3 : [Nat] = [3, 2, 1];

assert Array.equal(arr1, arr2, Nat.equal);
assert not Array.equal(arr1, arr3, Nat.equal);
```

Unlike some languages, Motoko does not compare arrays by reference; instead, it uses proper element-by-element structural comparison.

## Transforming arrays

The [`Array`](https://mops.one/core/docs/Array) module in Motoko's core package contains built-in functions for mapping over elements, filtering values, and summing numerical arrays.

```motoko no-repl
import Array "mo:core/Array";

func transformArray() : [Nat] {
  let numbers : [Nat] = [1, 2, 3];
  Array.map<Nat, Nat>(numbers, func(x) { x * 2 });
};
transformArray();
```

## Nested immutable arrays example: Chessboard

To demonstrate nested immutable arrays, consider the following:

A chessboard is a fixed `8×8` grid. Using immutable arrays to represent the initial [state](../actors/state.md) of the board ensures that the setup remains unchanged, preventing accidental modifications. This is useful because the starting position of pieces in chess is fixed, and any changes should be intentional, such as when making a move. Immutable arrays provide stability and help maintain the integrity of the initial board [state](../actors/state.md).

```motoko no-repl
import Array "mo:core/Array";
import Debug "mo:core/Debug";

persistent actor Chess{

  func generateChessboard() : [[Text]] {
    let size : Nat = 8;

    let board : [[Text]] = Array.tabulate<[Text]>(size, func(r : Nat) : [Text] {
      Array.tabulate<Text>(size, func(c : Nat) : Text {
          switch (r, c) {
            case (0, 0) {"♜"}; case (0, 1) {"♞"}; case (0, 2) {"♝"}; case (0, 3) {"♛"}; case (0, 4) { "♚"};
            case (0, 5) {"♝"}; case (0, 6) {"♞"}; case (0, 7) {"♜"}; case (1, _) {"♟"}; // Pawns on row 1
            case (6, _) {"♙"}; // Pawns on row 6
            case (7, 0) {"♖"}; case (7, 1) {"♘"}; case (7, 2) {"♗"}; case (7, 3) {"♕"};
            case (7, 4) {"♔"}; case (7, 5) {"♗"}; case (7, 6) {"♘"}; case (7, 7) { "♖"}; case (_, _) {"."} // Empty squares
            }
          }
        )
      }
    );
    board
  };

  // Generate the immutable chessboard
  let chessboard : [[Text]] = generateChessboard();

  // Display the board
  for (row in chessboard.values()) {
    let rowText = Array.foldLeft<Text, Text>(row, "", func(acc, square) = acc # square # " ");
    Debug.print(rowText)
  }
};
```

The function `Array.tabulate(size, f)` creates an array of `size` elements, populated so that element `i` contains the value of `f(i)`.

The function `Array.foldLeft` combines the squares in the row into a single text string, which can then be printed.

:::note
`Array.foldRight` is also available.
:::

``` md
♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜
♟ ♟ ♟ ♟ ♟ ♟ ♟ ♟
. . . . . . . .
. . . . . . . .
. . . . . . . .
. . . . . . . .
♙ ♙ ♙ ♙ ♙ ♙ ♙ ♙
♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖
```

## Subtyping

Immutable arrays also support subtyping.

For example, this means that, since `Nat` is a subtype of `Int`, the array type `[Nat]` is also a subtype of the array type `[Int]`.
Similarly, `[WeekDay]` <: `[Day]`, `[ {x : Nat; y: Nat; z : Nat } ] <: [ {x : Nat, y: Nat} ]` and so on.

For safety reasons, mutable arrays do not support subtyping. This is because the entries of a mutable array can also be written, not just read.

## Resources

- [`Array`](https://mops.one/core/docs/Array)
- [`Iter`](https://mops.one/core/docs/Iter)

