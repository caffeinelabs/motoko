---
title: "Contextual dot notation"
description: "Using contextual dot notation to call module functions with method-like syntax in Motoko."
sidebar:
  order: 10
---

Contextual dot notation is a language feature that allows you to call functions from modules using object-oriented style syntax, where a value appears as the receiver of a method call. This feature bridges the gap between Motoko's procedural and object-oriented programming styles.

## Overview

In Motoko, there are two main approaches to organizing and calling related functions: the object-oriented approach using classes and methods, and the procedural approach using modules and functions. Contextual dot notation allows you to use familiar method-like syntax for module functions, improving code readability and enabling better IDE support for code completion.

### The problem with traditional functional style

Consider a common operation on data structures. Without contextual dot notation, you would write:

```motoko no-repl
import Array "mo:core/Array";

let numbers = [1, 2, 3, 4, 5];
let doubled = Array.map(numbers, func(n) { n * 2 });
```

This functional style, while powerful, has some drawbacks:

- Code completion and IDE support are less effective because the function name comes first
- It reads "backwards" compared to how many developers think about operations
- The receiver value is separated from the operation by the module name and function

### Contextual dot notation syntax

With contextual dot notation, you can rewrite the same code as:

```motoko no-repl
import Array "mo:core/Array";

let numbers = [1, 2, 3, 4, 5];
let doubled = numbers.map(func(n) { n * 2 });
```

This reads more naturally: "take numbers and map over them". The IDE can also provide better code completion since it knows about all available operations for that type.

## How it works

Contextual dot notation works by allowing a module function to be called using dot notation syntax if its first parameter is of the appropriate type. The compiler treats `value.function(args)` as syntactic sugar for `Module.function(value, args)`.

### Requirements for contextual dot notation

For a function to be usable with contextual dot notation, it must:

1. be defined in a module (not a class method or object method),
2. have the first parameter named `self`
3. be publicly exported from its module.

The self parameter is indicated by its position as the first parameter and its type matching the value it's called on.

### Example: Using contextual dot notation

Here's a more comprehensive example using the `Array` module:

```motoko no-repl
import Array "mo:core/Array";
import Nat "mo:core/Nat";

let numbers = [1, 2, 3, 4];

// Traditional functional style
let doubled1 = Array.map(numbers, func(n) { n * 2 });

// Using contextual dot notation
let doubled2 = numbers.map(func(n) { n * 2 });

// Both produce the same result
assert Array.equal(doubled1, doubled2, Nat.equal);
```

## Enabling contextual dot notation in modules

When defining your own modules, you can make functions available through contextual dot notation. The first parameter of your function acts as the implicit receiver.

### Defining a contextual method

Here's how to define a module that supports contextual dot notation:

```motoko no-repl
module TextExt {
  // This function can be called as "str.uppercase()" due to contextual dot notation
  public func uppercase(self : Text) : Text {
    // Implementation here
  };

  public func lowercase(self : Text) : Text {
    // Implementation here
  };

  public func contains(self : Text, substring : Text) : Bool {
    // Implementation here
  };
}
```

You would then use these functions as:

```motoko no-repl
import TextExt "mo:text-utils/TextExt";

let message = "Hello World";
let upper = message.uppercase();
let lower = message.lowercase();
let found = message.contains("World");
```

### Naming conventions

While any function can use contextual dot notation based on its first parameter type, consider these guidelines:

- Use verb-based names for transformation and query operations: `map`, `filter`, `find`, `contains`, `replace`
- Use noun-based names for accessor or constructor operations: `size`, `length`, `keys`, `values`
- Avoid overly generic names that might be ambiguous across different types

## Contextual dot notation with generics

Contextual dot notation works seamlessly with generic types:

```motoko no-repl
import Array "mo:core/Array";

// These work with any type T
let naturals : [Nat] = [1, 2, 3];
let doubled = naturals.map(func(n) { n * 2 });

let texts : [Text] = ["a", "b", "c"];
let uppercased = texts.map(func(t) { /* convert to uppercase */ });
```

## Compiler warnings and best practices

The Motoko compiler can optionally warn you about opportunities to use contextual dot notation. You can enable this with the `-W M0236` flag:

```bash
moc -W M0236 myfile.mo
```

This helps you maintain consistent coding style across your project.

### When to use contextual dot notation

- **Use it** when the syntax is clearer and more readable
- **Use it** for commonly used operations like `map`, `filter`, `sort`, `find`
- **Consider the context** - in complex expressions, the functional style may be clearer
- **Avoid it** for less common or specialized operations where the module name provides important semantic information

## Limitations and considerations

Contextual dot notation has some intentional limitations:

- It requires the receiving value to be the first parameter, named `self`.
- Any function must be declared in a module that is imported or otherwise in scope: function in object, actors or nested modules are not considered.
- If there is more than one available module function, and none is more general than all the others, the call is considered ambigious and rejected at compile-time.
- The feature is purely syntactic - there is no runtime overhead


## See also

- [Modules and imports](modules-imports)
- [Language reference](../language-manual#dotted-function-calls)