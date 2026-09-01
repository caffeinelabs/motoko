---
title: "Options"
description: "Options provide a structured way to represent values that may or may not be present."
sidebar:
  order: 10
---

| Type    | Syntax                  | Purpose                          | Application                              |
|---------|-------------------------|----------------------------------|------------------------------------------------|
| Option  | `?T`                     | Represents a value that may be missing. | When an operation might return **no result**. |

Options provide a structured way to represent values that may or may not be present. Instead of using `null` directly, the option type enforces explicit handling, making programs safer and reducing unexpected failures.

An option is defined using `?` followed by the type of the value it can hold.

```motoko no-repl
var username : ?Text = null;
username;
```

`username` is an optional [`Text`](https://mops.one/core/docs/Text) value that starts as `null` (no username set).

:::note[Null semantics]

The constant `null` is the sole value of Motoko’s trivial `Null` type. It also represents the absence of a value in an optional type (`?T`). In this role, `null` is similar to `None` in Rust and Scala, and to `Nothing` in Haskell. Likewise, Motoko’s optional type `?T` is conceptually similar to `Option<T>` in Rust, `Option[T]` in Scala, and `Maybe T` in Haskell. In all these languages, the type system enforces explicit handling of missing values by representing optionality through a dedicated type.

:::

When a Motoko value has type `?T`, it is either `null` or contains a value, written as `?value` (note the leading `?`). The fundamental way to access this value is by using a `switch` expression, which explicitly handles both cases:

```motoko no-repl
func displayName(option : ?Text) : Text {
  switch option {
    case (?user) { user };
    case null { "Guest" };
  }
};
displayName(username);
```

Sometimes, the verbosity of a `switch` expression can make code harder to read. To improve readability, Motoko provides additional constructs for working with option types.

### Checking for presence

To determine if an option contains a value, function `Option.isSome` returns `true` if its argument is not `null`.

```motoko no-repl
import Option "mo:core/Option";
import Debug "mo:core/Debug";

let value : ?Nat = ?5;
if (Option.isSome(value)) {
  Debug.print("Value is present.");
}
```

By leveraging the `Option` module, handling optional values becomes more concise and expressive, reducing the need for explicit [`switch`](../control-flow/switch.md) statements.

### Providing default values

Instead of manually handling `null` cases with [pattern matching](../pattern-matching.md), `Option.get` allows for cleaner fallback logic to ensure that missing values are safely replaced with a default.

```motoko no-repl
import Option "mo:core/Option";

Option.get(username, "Guest"); // "Guest" if username is null
```

### Using options for error handling

Options can be used to catch expected failures instead of calling a [`trap`](../basic-syntax/traps.md), making a function return `null` when it encounters an invalid input.

```motoko no-repl
func safeDivide(a : Int, b : Int) : ?Int {
  if (b == 0) null else ?(a / b);
};

let result1 = safeDivide(10, 2); // ?5
safeDivide(10, 0); // null
```

Another way to extract values from option types is by using the `let ... else` pattern. This approach can be preferable to a `switch` expression for brevity and clarity. However, it only applies when the `else` branch can redirect control flow, such as returning early or throwing an error, if the value does not match the pattern in the `let`.

For example, here’s a simple implementation of an option helper:

```motoko no-repl
func get<T>(option : ?T, defaultValue : T) : T {
  let ?value = option else return defaultValue;
  return value;
};

assert get(?"A", "B") == "A";
assert get(null, "B") == "B";
```

The `let` statement matches the option against the pattern `?value`, extracting the contained value if present. If the option is `null`, the pattern fails to match and the `else` branch is executed, typically to exit the function early or return a default value.

The same logic can be expressed using a `switch`, though the result is more verbose and introduces an additional level of nesting:

```motoko no-repl
func get<T>(option : ?T, defaultValue : T) : T {
  switch option {
    case null defaultValue;
    case (?value) value;
  }
};
```

Although convenient for option patterns, `let-else` also works with other types of patterns.

### Applying transformations to options

The `Option.map` function applies a transformation only if the value is present.

```motoko no-repl
import Option "mo:core/Option";

let number : ?Nat = ?10;
Option.map<Nat, Nat>(number, func(x : Nat) = x * 2); // ?20
```

In this example, if `number` is `null`, `map` ensures the result remains `null` instead of performing an invalid operation.

### Applying an optional function

Sometimes, both the function and value are optional. `Option.apply` calls a function only if both are present. This is useful when chaining optional operations that may return `null`.

```motoko no-repl
import Option "mo:core/Option";

let maybeIncrement : ?(Nat -> Nat) = ?(func x: Nat = x + 1);
let maybeValue : ?Nat = ?10;

Option.apply<Nat, Nat>(maybeValue, maybeIncrement); // ?11
```

If either `maybeFunction` or `maybeValue` is `null`, the result remains `null`.

### Combining multiple optional values

When working with multiple optional values, using `Option.chain` processes them safely without unnecessary `switch` statements.

```motoko no-repl
import Option "mo:core/Option";

let firstName : ?Text = ?"Motoko";
let lastName : ?Text = ?"Ghost";

func combineNames(f : Text, l : Text) : Text {
    f # " " # l;
};

Option.chain<Text, Text>(firstName, func (first : Text) {
  Option.map<Text, Text>(lastName, func(last : Text) { combineNames(first, last) });
});

// ?("Motoko Ghost")
```

## Option blocks (`do ?`)

Option blocks in Motoko use the syntax `do ? <block>` to work with optional values of type `?T` without requiring nested `switch` statements. When the `<block>` evaluates to a value of type `T`, the entire `do ?` expression returns a value of type `?T`. Crucially, it allows early exits from the block when encountering `null`.

Within a `do ? <block>`, the `!` operator is used to unwrap values of unrelated option types (e.g., `?U`). When evaluating an expression `<exp> !`, if `<exp>` results in `null`, control immediately exits the `do ?` block with value `null`. Otherwise, it unwraps the value `?v` and continues with `v` of type `U`.

The `do ? <block>` construct is similar to the `?` operator in Rust, providing a concise and expressive way to propagate `null` values.

```motoko no-repl
 // Returns the sum of optional values `n` and `m` or `null`, if either is `null`
func addOpt(n : ?Nat, m : ?Nat) : ?Nat {
  do ? {
    n! + m!
  }
}
```

The following example defines a simple function that evaluates expressions built from natural numbers, division, and a zero test, encoded as a variant type:

```motoko no-repl
type Exp = {#Lit : Nat; #Div : (Exp, Exp); #If : (Exp, Exp, Exp)};
func eval(e : Exp) : ? Nat {
  do ? {
    switch e {
      case (#Lit n) { n };
      case (#Div (e1, e2)) {
        let v1 = eval e1 !;  // If eval e1 returns null, exit with null
        let v2 = eval e2 !;  // If eval e2 returns null, exit with null
        if (v2 == 0)
          null !  // Explicitly exit with null for division by zero
        else v1 / v2
        };
      case (#If (e1, e2, e3)) {
        if (eval e1 ! == 0)  // Unwrap and check if zero
          eval e2 !  // Return result of e2 (or null if it's null)
        else
          eval e3 !  // Return result of e3 (or null if it's null)
      };
    };
  };
};

let expr : Exp = #If(
  #Div(#Lit 10, #Lit 2),   // 10 / 2 = 5 (non-zero, so evaluate e3)
  #Lit 0,                  // e2 (ignored because e1 ≠ 0)
  #Div(#Lit 6, #Lit 3)     // e3 → 6 / 3 = 2
);

eval(expr);
```

To guard against division by 0 without trapping, the `eval` function returns an option result, using `null` to indicate failure.

Each recursive call is checked for `null` using `!`, immediately exiting the outer `do ?` block and then the function itself, when an intermediate result is `null`.

## Resources

- [`Option`](https://mops.one/core/docs/Option)


