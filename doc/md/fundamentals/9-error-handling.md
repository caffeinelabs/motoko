---
sidebar_position: 9
---

# Error handling

Using `Option` or `Result` is the preferred way of signaling errors in Motoko. They work in both synchronous and asynchronous contexts and make your APIs safer to use by encouraging clients to consider the error cases as well as the success cases. Exceptions should only be used to signal unexpected error states.

In addition to explicit error handling, Motoko provides traps and assertions for dealing with execution errors.

## Error reporting with `Option` types

When a function might either return a value of type `A` or signal an error, it can return an option type `?A`. In this pattern, `null` is used to indicate an error or missing result, while `?value` wraps a successful outcome.

In the following example, if the `markDone` function sometimes fails and returns a number of seconds on success, its return type would be `async ?Seconds`. This makes it clear to callers that the result may be absent and must be handled safely.


Function definition:

``` motoko no-repl file=../examples/todo-error.mo#L49-L58
```

Function callsite:

``` motoko no-repl file=../examples/todo-error.mo#L117-L126
```

The main drawback of using option types to signal errors is that all failures are represented by a single, non-descriptive `null` value. This means important information about why something failed is lost. As a result, the only message the program can show the user might be something vague like `"Something went wrong."`

For this reason, option types should only be used for errors when there's just one clear reason for failure, and it can be easily understood at the callsite.

## Error reporting with `Result` types

While options are a built-in type, the `Result` is defined as a variant type like so:

``` motoko no-repl
type Result<Ok, Err> = { #ok : Ok; #err : Err }
```

Unlike option types, the Result type includes a second type parameter `Err` which allows you to specify exactly what kind of error occurred. This makes error handling more informative and flexible.

``` motoko no-repl file=../examples/todo-error.mo#L59-L59
```

The previous example can be revised to use `Result` types:

Function definition:

``` motoko no-repl file=../examples/todo-error.mo#L62-L76
```

Function callsite:

``` motoko no-repl file=../examples/todo-error.mo#L128-L141
```

## Pattern matching

The most common way of working with `Option` and `Result` is to use pattern matching. If you have a value of type `?Text`, you can use the `switch` keyword to access the potential [`Text`](../core/Text) contents:

``` motoko no-repl file=../examples/error-examples.mo#L3-L10
```

Motoko does not let you access the optional value without also considering the case that it is missing.

With a `Result` type, you can use pattern matching to handle both success and error cases. Unlike option types, the `#err` case carries detailed information about what went wrong, not just a `null` value.

``` motoko no-repl file=../examples/error-examples.mo#L12-L19
```

Sometimes you need to convert between `Option` and `Result` types. For example, a HashMap lookup returns `null` on failure (an `Option`), but if the caller has more context, they can turn that failure into a meaningful `Result` with an error message. On the other hand, sometimes you don’t need the extra detail from a `Result` and just want to convert any error (`#err`) into `null`.

The [core](https://github.com/dfinity/motoko-core) package provides `fromOption` and `toOption` functions in the `Result` module that make converting between these two types easy.

## Error reporting with `Error` (asynchronous errors)

Another way to handle errors in Motoko is with asynchronous `Error` handling, which is a limited form of exception handling. These errors can only be thrown and caught in asynchronous contexts, like inside `shared` functions or `async` blocks. Regular (non-`shared`) functions can’t use this kind of structured error handling.

This means you can `throw` an `Error` to exit a shared function early, and callers can `try` to run that code and `catch` the error if it happens. However, you can only use `throw` and `catch` inside asynchronous contexts.

Asynchronous `Error`s are best reserved for unexpected failures that you don’t expect most callers to handle. If you want the caller to handle possible failures explicitly, it’s better to use `Result` in your function’s return type.

Here’s how the `markDone` function might look using exceptions:

Function definition:

``` motoko no-repl file=../examples/todo-error.mo#L78-L92
```

Function callsite:

``` motoko no-repl file=../examples/todo-error.mo#L143-L150
```

## Traps

Traps immediately stop execution and roll back [state](../fundamentals/2-actors/2-state.md). They are used for fatal errors that cannot be recovered.

```motoko no-repl
import Runtime "mo:core/Runtime";

func divide(a : Nat, b : Nat) : Nat {
    if (b == 0) {
        Runtime.trap("Cannot divide by zero");
    };
    return a / b;
};
```

## Assertions

Assertions enforce expected conditions. If the condition is false, they introduce a trap but are not errors themselves.

```motoko no-repl
func validateAge(age : Nat) : () {
    assert(age >= 18);  // Traps if age is below 18
};
```

## How not to handle errors

Using sentinel values to report errors is generally a bad practice and strongly discouraged. For example, you might have `markDone` return `-1` to indicate failure. In that case, the caller has to remember to check for that special value every time. It’s easy to forget, which can cause errors to go unnoticed or be detected too late.

