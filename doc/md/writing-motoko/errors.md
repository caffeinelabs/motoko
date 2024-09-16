---
sidebar_position: 9
---

# Error handling

## Overview

There are three primary ways to represent and handle errors values in Motoko:

-   Option values with a non-informative `null` value that indicates some error.

-   `Result` variants with a descriptive `#err value` providing more information about the error.

-   [`Error`](../base/Error.md) values that, in an asynchronous context, can be thrown and caught similar to exceptions and contain a numeric code and message.

## Example

Consider building an API for a to-do application that wants to expose a function allowing users to mark one of their tasks’s as "Done". This simple example will accept a `TodoId` object and return an [`Int`](../base/Int.md) that represents how many seconds the to-do has been open. This example assumes that it is running in an actor, which returns an async value:

``` motoko no-repl
func markDone(id : TodoId) : async Int
```

The full application example can be found below:

``` motoko no-repl file=../examples/todo-error.mo#L1-L6
```

``` motoko no-repl file=../examples/todo-error.mo#L10-L37
```

In this example, there are conditions under which marking a to-do as "Done" fails:

-   The `id` could reference a non-existing to-do.

-   The to-do might already be marked as done.

Let's look at the different ways to communicate these errors in Motoko and slowly improve the example API.

## Option/result

Using `Option` or `Result` is the preferred way of signaling errors in Motoko. They work in both synchronous and asynchronous contexts and make your APIs safer to use by encouraging clients to consider the error cases as well as the success cases. Exceptions should only be used to signal unexpected error states.

### Error reporting with `Option` types

A function that wants to return a value of type `A` or signal an error can return a value of option type `?A` and use the `null` value to designate the error. In the above example this means the `markDone` function returns an `async ?Seconds`:

Definition:

``` motoko no-repl file=../examples/todo-error.mo#L49-L58
```

Callsite:

``` motoko no-repl file=../examples/todo-error.mo#L117-L126
```

The main drawback of this approach is that it conflates all possible errors with a single, non-informative `null` value. The callsite might be interested in why marking a `Todo` as done has failed, but that information is lost by then, which means we can only tell the user that `"Something went wrong."`.

Returning option values to signal errors should only be used if there just one possible reason for the failure and that reason can be easily determined at the callsite. One example of a good use case for this is a HashMap lookup failing.

### Error reporting with `Result` types

While options are a built-in type, the `Result` is defined as a variant type like so:

``` motoko no-repl
type Result<Ok, Err> = { #ok : Ok; #err : Err }
```

Because of the second type parameter, `Err`, the `Result` type lets you select the type used to describe errors. Define a `TodoError` type that the `markDone` function will use to signal errors:

``` motoko no-repl file=../examples/todo-error.mo#L60-L60
```

The original example is now revised as:

Definition:

``` motoko no-repl file=../examples/todo-error.mo#L62-L76
```

Callsite:

``` motoko no-repl file=../examples/todo-error.mo#L128-L141
```

### Pattern matching

The first and most common way of working with `Option` and `Result` is to use pattern matching. If you have a value of type `?Text`, you can use the `switch` keyword to access the potential [`Text`](../base/Text.md) contents:

``` motoko no-repl file=../examples/error-examples.mo#L3-L10
```

Motoko does not let you access the optional value without also considering the case that it is missing.

In the case of a `Result`, you can also use pattern matching with the difference that you also get an informative value, not just `null`, in the `#err` case:

``` motoko no-repl file=../examples/error-examples.mo#L12-L19
```

### Higher-order functions

Pattern matching can become tedious and verbose, especially when dealing with multiple optional values. The [base](https://github.com/dfinity/motoko-base) library exposes a collection of higher-order functions from the `Option` and `Result` modules to improve the ergonomics of error handling.

Sometimes you’ll want to move between `Option` and `Result`. A Hashmap lookup returns `null` on failure, but maybe the caller has more context and can turn that lookup failure into a meaningful `Result`. Other times you don’t need the additional information a `Result` provides and just want to convert all `#err` cases into `null`. For these situations [base](https://github.com/dfinity/motoko-base) provides the `fromOption` and `toOption` functions in the `Result` module.

## Asynchronous errors

The last way of dealing with errors in Motoko is to use asynchronous [`Error`](../base/Error.md) handling, a restricted form of the exception handling familiar from other languages. Motoko error values can only be thrown and caught in asynchronous contexts, typically the body of a `shared` function or `async` expression. Non-`shared` functions cannot employ structured error handling. This means you can exit a shared function by `throw`ing an [`Error`](../base/Error.md) value and `try` some code calling a shared function on another actor. In this workflow, you can `catch` the failure as a result of type [`Error`](../base/Error.md), but you can’t use these error handling constructs outside of an asynchronous context.

Asynchronous [`Error`](../base/Error.md)s should generally only be used to signal unexpected failures that you cannot recover from and that you don’t expect many consumers of your API to handle. If a failure should be handled by your caller, you should make it explicit in your signature by returning a `Result` instead. For completeness, here is the `markDone` example with exceptions:

Definition:

``` motoko no-repl file=../examples/todo-error.mo#L78-L92
```

Callsite:

``` motoko no-repl file=../examples/todo-error.mo#L143-L150
```

## Using try/finally

A `finally` clause can be used within a `try/catch` error handling expression that facilitates control-flow expression cleanups, resource deallocation, or rolling back temporary state changes. The `finally` clause is optional, and when used, the `catch` clause may be omitted. Any uncaught error from the `try` block will be propagated after the `finally` block has executed.

:::info

`try/finally` is supported in `moc` `v0.12.0` and newer, and `dfx` `v0.24.0` and newer.

:::

`try/finally` must be used within an async expression or in the body of a shared function. Before using `try/finally`, please review the [security best practices](https://internetcomputer.org/docs/current/developer-docs/security/security-best-practices/inter-canister-calls#recommendation) for using this syntax.

``` motoko no-repl file=../examples/try-finally.mo
```

Inside the `try` block, include code that may throw an error. In the `finally` block, include code that should be executed whether an error was thrown or not. Code within the `finally` block should not trap and should terminate promptly. If a `finally` block were to trap, it may prevent a future upgrade to the canister.

Learn more about [`try/finally`](https://internetcomputer.org/docs/current/motoko/main/reference/language-manual#try).

### How not to handle errors

A generally poor way of reporting errors is through the use of a sentinel value. For example, for your `markDone` function, you might decide to use the value `-1` to signal that something failed. The callsite then has to check the return value against this special value and report the error. It's easy to not check for that error condition and continue to work with that value in the code. This can lead to delayed or even missing error detection and is strongly discouraged.

Definition:

``` motoko no-repl file=../examples/todo-error.mo#L38-L47
```

Callsite:

``` motoko no-repl file=../examples/todo-error.mo#L108-L115
```

<img src="https://github.com/user-attachments/assets/844ca364-4d71-42b3-aaec-4a6c3509ee2e" alt="Logo" width="150" height="150" />