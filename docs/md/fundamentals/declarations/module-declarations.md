---
title: "Module declarations"
description: "In Motoko, a module is a collection of related types, values, and functions grouped under a single namespace."
sidebar:
  order: 7
---

In Motoko, a **module** is a collection of related types, values, and functions grouped under a single namespace. Unlike actors and objects, modules cannot declare mutable state or have side effects during their construction. This restriction makes them ideal for defining code libraries, since you don’t need to worry about the side effects or state implications of importing the same library multiple times or removing an unused one.

Modules are mainly used to build libraries, such as those in the [core package](https://mops.one/core/docs/index) or packages available through [Mops, the Motoko package manager](https://mops.one).

A module in Motoko can define:

- Public types.
- Public functions, both synchronous and asynchronous.
- Private types and internal logic that is not exposed outside the namespace.
- Nested modules.

Modules provide encapsulation and structure, allowing code to be organized and reused across different parts of an application.

```motoko no-repl
import Array "mo:core/Array";

module Matrix {
  // Define what a matrix is - a 2D array of Nat
  public type Matrix = [[Nat]];

  // Create a matrix filled with a default value
  public func init(rows : Nat, cols : Nat, default : Nat) : Matrix {
    Array.tabulate<[Nat]>(rows, func (i : Nat) {
      Array.tabulate<Nat>(cols, func (j : Nat) {
        default
      })
    })
  };

  // Create an identity matrix (ones on diagonal, zeros elsewhere)
  public func identity(n : Nat) : Matrix {
    Array.tabulate<[Nat]>(n, func (i : Nat) {
      Array.tabulate<Nat>(n, func (j : Nat) {
        if (i == j) { 1 } else { 0 }
      })
    })
  };

  // Transpose a matrix (flip rows and columns)
  public func transpose(m : Matrix) : Matrix {
    if (m.size() == 0) return [];
    let rows = m.size();
    let cols = m[0].size();
    Array.tabulate<[Nat]>(cols, func (j: Nat) {
      Array.tabulate<Nat>(rows, func (i:Nat) {
        m[i][j]
      })
    })
  };
};
```

This module declares:

- A public type `Matrix.Matrix` for representing a 2D array of natural numbers `[[Nat]]`.
- Functions `Matrix.init()`, `Matrix.identity()`, and `Matrix.transpose()` for matrix operations.

:::note[Functional programming style]
This module follows a functional programming approach. All operations return new matrices instead of modifying existing ones.
:::


## Module restrictions

Modules cannot contain mutable state (`var`) or non-static expressions.

```motoko no-repl
import Debug "mo:core/Debug";
module myModule {
  var x = 0; // Not allowed, non-static expression in library
  Debug.print("hello"); // Not allowed, side-effect in library
}
```

If you need mutable state, use an `actor` or an `object` inside an actor instead:

```motoko no-repl
persistent actor {
  var x = 0; // allowed because actors can have state
}
```

A module can declare classes that use state, provided it doesn't instantiate those classes in its body.

For example, a module can define a class of stateful `Counters` :

```motoko no-repl
module Counters {
   public class Counter() {
      var count : Nat = 0;
      public func inc() : Nat {
        count += 1;
        count
      }
   }
}
```

## Using modules

Library modules are accessed using `import` declarations. As a general convention, it's good style to reuse the file name as the import name.
However, the importing code is free to assign any name to the imported module and may choose a different name to avoid naming conflicts.

Public types and values within a module are accessed using dot notation, the same syntax used for accessing fields of objects and actors. The module name acts as a namespace, helping to organize related functionality and avoid name clashes across the codebase.

```motoko no-repl
import Matrix "Matrix";

// Create a 3x3 matrix filled with zeros
let zeros = Matrix.init(3, 3, 0);

// Create a 4x4 identity matrix
let id4 = Matrix.identity(4);

// Transpose a matrix
let transposed = Matrix.transpose([[1, 2, 3], [4, 5, 6]]);
// Result: [[1, 4], [2, 5], [3, 6]]

// Use the Matrix type from the module
let myMatrix : Matrix.Matrix = [[1, 2], [3, 4]];

// Chain operations together
let result = Matrix.transpose(Matrix.identity(3));
```

If you'd prefer to avoid dot notation, you can also import individual values directly from a module using pattern matching in the import statement.

This allows you to bind specific names from the module into your local scope, making the code more concise and readable when those values are used frequently.

```motoko no-repl
import {init; identity} "Matrix";

let zero = init(3, 3, 0);
let id4 = identity(4);
```

### Module classes

A module class can be used to produce multiple modules with different configurations.

<!-- TODO: a better example would have type fields, e.g. a simple vector lib -->

```motoko no-repl
import Debug "mo:core/Debug";

module class ExchangeRate(baseRate : Float) {
    public func convert(amount : Nat) : Float {
        return Float.fromInt(amount) * baseRate;
    };
};

// Creating different currency converters
let usdConverter = ExchangeRate(1.1);
let eurConverter = ExchangeRate(0.9);

Debug.print(debug_show(Float.toText(usdConverter.convert(100))));  // "110.0"
Debug.print(debug_show(Float.toText(eurConverter.convert(100))));  // "90.0"
```