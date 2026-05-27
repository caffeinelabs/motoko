---
title: "Results"
description: "While options are a built-in type, the Result is defined as a variant type:"
sidebar:
  order: 11
---

| Type    | Syntax                  | Purpose                          | Application                                 |
|---------|-------------------------|----------------------------------|------------------------------------------------|
| Result  | `Result.Result<T, E>`  where `T` is the success type and `E` is the error type.  | Represents success or failure.   | When an operation can either **succeed or fail**. |

While options are a built-in type, the `Result` is defined as a variant type:

```motoko no-repl
type Result<Ok, Err> = { #ok : Ok; #err : Err }
```

Because of the second type parameter `Err`, the `Result` type lets you select the type used to describe errors.

Here’s a simple function that validates a username. If it's non-empty, it will return a greeting of type `Text`. If not, it will return an error of type `Text`.

```motoko no-repl
import Result "mo:core/Result";

func greet(name : Text) : Result.Result<Text, Text> {
  if (name.size() == 0) #err "Name is empty" else #ok ("Hello " # name);
};
```

## Pattern matching with `Result`

When a Motoko value has type `Result<T, E>`, it is either a success, written `#ok value`, or a failure, written `#err error`. The fundamental way to access the contents is to use a `switch` expression that explicitly handles both cases.

### `switch`

```motoko no-repl
import Result "mo:core/Result";

func greet(name : Text) : Result.Result<Text, Text> {
  if (name.size() == 0) #err "Name is empty" else #ok ("Hello " # name);
};

let result = greet("Motoko");
switch (result){
  case (#ok message) {
    message
  };
  case (#err error) {
    error
  };
} // "Hello Motoko"
```

The verbosity of `switch` expressions can make code harder to read, so Motoko also offers additional ways to handle `Result`.

### `let else`

Values can be extracted from `Result` using the `let ... else` pattern. This can be preferable to a `switch` expression when only the success `case` is needed, and the else branch can cleanly handle or exit on failure. It allows concise early returns or alternative flows when the result is `#err`.

```motoko no-repl
import Result "mo:core/Result";

func greet(name : Text) : Result.Result<Text, Text> {
  if (name.size() == 0) #err "Name is empty" else #ok ("Hello " # name);
};

func safeGreet(name : Text) : Text {
  let #ok greeting = greet(name) else return "Default greeting: Hi!";
  greeting
};

safeGreet("");      // returns "Default greeting: Hi!"
```

In `safeGreet`, the `else` branch of the `let` expression allows the function to exit early with a default greeting when the result of `greet` is an error (i.e., does not match `#ok greeting` but instead returns `#err ...`). However, this pattern does not provide access to the actual error value.

If you need to handle or inspect the error, it's better to use a `switch` statement, which gives you explicit access to both `#ok` and `#err` cases.

## Resources

- [`Result`](https://mops.one/core/docs/Result)
