---
title: "Actors & async data"
description: "The actor programming model was designed to solve concurrency issues by encapsulating state and computation within independent units called actors."
sidebar:
  order: 1
---

The actor programming model was designed to solve concurrency issues by encapsulating [state](./state.md) and computation within independent units called **actors**.

The actor model is built on four key principles:

* **Isolation**: Actors are isolated and communicate solely through message passing.
* **Concurrency**: Actors can receive and process messages concurrently.
* **Fault tolerance**: Actors operate independently and can fail without affecting others.
* **Location transparency**: Actors can reside on any machine within a distributed system.

In Motoko, actors have dedicated syntax and type rules that support asynchronous, message-based communication:

* **Shared functions** handle messaging between actors. These functions return **futures** (values of type `async T`) and are accessible to remote callers. Shared functions come with restrictions. Both their arguments and return values must be **shared types**, a subset of types that includes immutable data, actor references, and shared function references, but excludes local function references and mutable data (like `var` or mutable arrays).

* A **future**, `f`, has the type `async T` and represents a value of type `T` that will be available later.

* To retrieve the result of a future, use `await f`, which pauses execution until the future is resolved and returns a value of type `T`. `await? f` can be used when the the future is likely resolved and the state commit semantics is irrelevant.

* These restrictions help prevent shared mutable state from being introduced via messaging. Only immutable, shared data can be sent between actors through shared functions.

* All mutable state should be encapsulated within the actor or actor class. A Motoko source file defining an actor typically starts with `import` statements, followed by the `actor` or `actor class` declaration.

## `async` / `await`

Motoko, like many other languages, offers `async` and `await` to support convenient programming with asynchronous functions and computations.

When you run an asynchronous expression, like calling a shared function or creating a local `async` block, it returns a **future**, which is a placeholder for a result that will be available later.

Instead of making the caller wait, the message is sent and queued, and the future is immediately returned. The caller can continue doing other work and later use `await` to pause until the future is ready and get the result.

You can also attach extra information to an async call using a prefixed parenthetical in the form `(base with attr₁ = v₁; attr₂ = v₂; …)` where `base` is an optional record containing (e.g., default) attributes. Accepted attributes are currently `cycles : Nat`, specifying the amount of cycles to be sent along with the message, and `timeout : Nat32` to modify the deadline and restrict the time span while the receiver can reply.

The combination of `async`/`await` constructs simplifies asynchronous programming  by allowing `await`s to be embedded within ordinary sequential code, without requiring tricky management of asynchronous callbacks.

## Traps and commit points

A trap is a non-recoverable runtime failure in Motoko, caused by errors such as:

* Division by zero.
* Out-of-bounds array access.
* Numeric overflow.
* Exceeding cycle limits.
* Failing an assertion.
* An explicit call to `Debug.trap()`. <!-- FUTURE: `Runtime.trap() in new base -->

When a shared function executes without evaluating an `await` expression, it never suspend and thus runs atomically, meaning its execution cannot be interleaved with that of another message. Functions that don't contain any `await` expressions are syntactically atomic and guarantee interference-free execution from start to finish.

### Commit points

If an atomic shared function traps during execution, it has no visible effect. Any state changes are reverted, and messages sent are revoked. This is because all changes are tentative during execution and only become permanent once a commit point is reached.

The commit points, where tentative state changes and message sends are irrevocably committed, are:

* Implicit exit from a shared function by producing a result.

* Explicit exit via `return` or `throw` expressions.

* Execution of an `await` expression, which suspends the function and commits all changes up to that point.

### Traps

A trap will only revoke changes made since the last commit point. In particular, in a non-atomic function that does multiple awaits, a trap will only revoke changes attempted since the last await. All preceding effects will have been committed and cannot be undone.

Consider the following stateful `Atomicity` actor:

```motoko no-repl file=<motokoExamples>/atomicity.mo
```

Calling the shared function `atomic()` results in an error because it traps before completing. Since the trap happens before any `await` or return, all changes are discarded. The variable `s` stays at 0, and `pinged` remains false. Even though `atomic()` calls `ping()`, that message is only queued and never sent because no commit point is reached.

Calling `nonAtomic()` also fails with an error, but the state is partially updated. In this case, `s` ends up as 3, and `pinged` is true. This happens because the first `await` commits all prior changes, including the message send. The second `await` causes another commit and suspends execution, allowing other messages to be processed before the trap occurs.


## Async functions

Here is an example program that uses async functions:

```motoko file=<motokoExamples>/counter-actor.mo
```

The `Counter` actor declares one field and three public, shared functions:

-   The field `count` is mutable, initialized to zero and implicitly `private`.

-   Function `inc()` asynchronously increments the counter and returns a future of type `async ()` for synchronization.

-   Function `read()` asynchronously reads the counter value and returns a future of type `async Nat` containing its value.

-   Function `bump()` asynchronously increments and reads the counter.

The only way to read or modify the state (`count`) of the `Counter` actor is through its shared functions.

## Using `await` to consume `async` futures

The caller of a shared function typically receives a future, a value of type `async T` for some `T`.

The only thing the caller can do with this future is wait for it to be completed by the producer, throw it away, or store it for later use.

To access the result of an `async` value, the receiver of the future uses an `await` expression.

For example, to use the result of `Counter.read()` above, we can first bind the future to an identifier `a`, and then `await a` to retrieve the underlying [`Nat`](https://mops.one/core/docs/Nat), `n`:

```motoko no-repl
let a : async Nat = Counter.read();
let n : Nat = await a;
```

The first line immediately receives a future of the counter value, but does not wait for it, and thus cannot use it as a natural number yet.

The second line `await`s this future and extracts the result, a natural number. This line may suspend execution until the future has been completed.

Typically, one rolls the two steps into one and just awaits an asynchronous call directly:

```motoko no-repl
let n : Nat = await Counter.read();
```

Unlike a local function call, which waits for the result before continuing, a shared function call returns a future immediately without blocking. Later, calling `await` on that future pauses the current task until the future is finished. When the future completes, `await` either returns the result or throws an error if the future ended with one.

If you `await` the same future again, it just returns the same result or error. Even if the future is already done, `await` will briefly suspend all pending state changes and outgoing messages. This means that you can rely on every `await` to commit state, whether its future is still in progress or already completed.

## Using `await?` to efficiently await concurrent futures

An `await` will always suspend execution and commit state, even if its future is already complete.

When several futures are issued in parallel and racing to complete, it can be more efficient to opt out of the unconditional behavior of `await` and immediately continue with a result when it is available:

```motoko no-repl
let a : async Nat = CounterA.read();
let b : async Nat = CounterB.read();
let sum : Nat = (await a) + (await? b);
```

Here the futures `a` and `b` are racing to complete, and it is likely that the first `await` on `a` will resume with `b` already completed.
Using `await? b` ensures that `b`'s result can be used immediately, if available, without an unnecessary suspension.

:::danger

Since a commit of global state may not happen when using `await?`, this construct should be only used when the commit can be safely omitted.

:::


## Using parentheticals to modify message send modalities

In the examples above, messages sent to the `Counter` actor do not include cycles and will never timeout when waiting for their results. However, you can change these behaviors by adding a parenthetical expression that modifies the message’s attributes.

To send cycles with a message, you can write:

```motoko no-repl
let a = (with cycles = 42_000_000) Counter.bump();
```

To set a timeout for awaiting the message result, which is useful when you want a best-effort response rather than a guaranteed one, you can write:

```motoko no-repl
let a = (with timeout = 25) Counter.bump();
```

You can also define a custom default set of attributes as a record and then extend it with additional attributes in the parenthetical:

```motoko no-repl
let boundedWait = { timeout = 25 };
let a = (boundedWait with cycles = 42_000_000) Counter.bump();
```

This approach lets you easily customize message sending with cycles and timeouts.

:::danger

A function that does not use `await` runs atomically, meaning nothing else can change the actor’s state while it’s running. But if the function uses `await`, it can be paused, and during that pause, other messages may change the actor’s state. It’s up to the programmer to handle these possible changes safely. However, any state changes made before the `await` are guaranteed to be saved.

For example, the implementation of `bump()` above is guaranteed to increment and read the value of `count`, in one atomic step. The following alternative implementation does not have the same semantics and allows another client of the actor to interfere with its operation.

```motoko no-repl
  public shared func bump() : async Nat {
    await inc();
    await read();
  };
```

Each `await` suspends execution, allowing an interloper to change the state of the actor. By design, the explicit `await`s make the potential points of interference clear to the reader.

:::

## Async actors

In Motoko, each communicating component is an actor, encapsulating its own state and behavior. Here's a simple three-line example that demonstrates basic actor usage:

```motoko no-repl
let result1 = service1.computeAnswer(params);
let result2 = service2.computeAnswer(params);
finalStep(await result1, await result2)
```

This program’s behavior can be summarized as:

1.  The program makes two requests (lines 1 and 2) to two distinct services, each implemented as a Motoko actor or canister smart contract implemented in some other language.

2.  The program waits for each result to be ready (line 3) using the keyword `await` on each result value.

3.  The program uses both results in the final step (line 3) by calling the `finalStep` function.

Services **interleave** their execution to reduce latency instead of waiting for each other. Without language support, this kind of interleaving quickly becomes complex and hard to manage.

Even with just a single async call, Motoko’s abstractions help keep the code clear. By using `await`, the programmer tells the compiler where interleaving can happen, avoiding the need to restructure logic to fit the system’s message-passing loop.

In other languages without these features, developers often need to use advanced patterns like callbacks and event handlers. This low-level, systems-style programming can be powerful but is error-prone, as it breaks high-level logic into scattered events and shared state.

### Example

To demonstrate how asynchronous actors work, consider the following example.

Customers place orders at a pizza restaurant, but the chef can only make one pizza at a time. Orders are taken **[asynchronously](./actors-async.md#async--await)**, meaning customers do not have to wait for previous orders to be completed before placing their own. However, each pizza is prepared sequentially. This is representative of an asynchronous actor.
<!-- TODO(FUTURE): It would be cleaner to use a Deque or Queue pushing new order to the end and popping the next order to make from the front. -->
```motoko no-repl
import Array "mo:core/Array";
import Text "mo:core/Text";

persistent actor PizzaParlor {
    var orders : [Text] = [];

    public shared func placeOrder(order : Text) : async Text {
        // Use Array.tabulate to create a new array with the additional element
        let newOrders = Array.tabulate<Text>(orders.size() + 1, func(i) {
            if (i < orders.size()) { orders[i] } else { order }
        });
        orders := newOrders;
        return "Order received: " # order;
    };

    public shared func makePizza() : async Text {
        if (orders.size() == 0) {
            return "No orders to make.";
        };

        let currentOrder = orders[0];
        // Use Array.filter to remove the first order, assuming orders are unique
        orders := Array.filter<Text>(orders, func(o) {
            not Text.equal(o, currentOrder)
        });
        return "Made a delicious " # currentOrder # " pizza!";
    };

    public query func getOrders() : async [Text] {
        return orders;
    };
}
```

## `async*` / `await*`

You can move asynchronous code into a local `async` function and replace repeated code with calls to it. Since these calls return futures, you need to `await` each one to get the result. However, this approach has drawbacks:

- Each call sends an extra message to the actor.

- Every call must be awaited, adding overhead.

- Each `await` suspends execution, increasing chances of interference from other concurrent messages.

To reduce the overhead and risks of extra `await`s, Motoko provides computation types, written as `async* T`. Like futures (`async T`), computations can represent asynchronous tasks.

An `async` expression creates a future by starting its execution immediately, while an `async*` expression creates a computation by delaying execution until needed. Similarly, `await` gets the result of a future, and `await*` gets the result of a computation by triggering its next step.

From a typing perspective, futures and computations are similar, but they behave differently at runtime. A future is a stateful object representing a scheduled asynchronous task, while a computation is an inactive value that describes a task.

When you use `await` on a future, it suspends the caller until the task finishes. But `await*` on a computation doesn’t suspend the caller; it immediately runs the computation like a normal function call.

This means that `await*` only causes suspension if the computation’s body itself uses a regular `await`. The `*` indicates that the computation might include zero or more `await` calls and so may be interleaved with other message executions.

You create an `async*` value by using an `async*` expression, but usually, it’s done by defining a local function that returns an `async*` type.

To get the result of an `async*` computation, you use `await*`.

:::danger

Use `async*` and `await*` carefully. In Motoko, a regular `await` is a commit point. State changes are saved before the function pauses.

`await*` is not a commit point because the computation it runs may not pause or commit at a predictable time. This means if a trap happens inside an `await*` computation, the actor’s state will roll back to the last commit point before the `await*`, not to the point of the `await*` itself.

:::

### Example

```motoko no-repl
persistent actor class (Logger : actor { log : Text -> async () }) {

  var logging = true;

  func maybeLog(msg : Text) : async* () {
    if (logging) { await Logger.log(msg) };
  };

  func doStuff() : async () {
    // do stuff
    await* maybeLog("Log entry #1");
    // do more stuff
    await* maybeLog("Log entry #2");
  }
}
```

## `try/finally`

The `try/finally` construct ensures that a block of code in the `finally` clause executes regardless of whether an exception occurs in the `try` block. This is particularly useful for cleanup operations, such as logging or finalizing an action, ensuring that necessary steps are taken even if an error interrupts execution.

In the `placeOrder` function below:

- The `try` block processes an order and appends it to the `orders` array.
- The `finally` block logs that the order has been processed, ensuring that this message is always printed, whether the function succeeds or fails.

```motoko no-repl
public shared func placeOrder(order : Text) : async Text {
    try {
      Debug.print("Processing order: " # order);

      let newOrders = Array.tabulate<Text>(
        orders.size() + 1,
        func(i) {
          if (i < orders.size()) {orders[i]} else {order}
        }
      );

      orders := newOrders;
      return "Order received: " # order
    } finally {
      Debug.print("Order processed : " # order)
    };

};
```

## `try/catch/finally`

A `catch` block can be inserted inside a `try/finally` expression to catch an error.

```motoko no-repl
    try {
    // Code that might throw an error
    } catch (e) {
    // Handle the error
    } finally {
    // Cleanup code that always runs
    }
```

When `catch` is used, the `finally` clause is optional. The `catch` block only catches errors in certain scenarios:

1. Explicit throws: When code in the `try` block explicitly throws an error using the `throw` keyword:

```motoko no-repl
    try {
        throw Error.reject("Intentional error");
    } catch (e) {
        // This will catch the explicitly thrown error
    }
```

2. Errors from awaited calls: If an `await` expression in the `try` block returns an error:

```motoko no-repl
    try {
        let result = await someAsyncFunction(); // If this returns an error
    } catch (e) {
        // The error will be caught here
    }
```

`catch` blocks **do not** catch errors in the following scenarios:

1. Local traps.
2. Pre-await traps in async functions.
3. Traps after `await`.


<img src="https://cdn-assets-eu.frontify.com/s3/frontify-enterprise-files-eu/eyJwYXRoIjoiZGZpbml0eVwvYWNjb3VudHNcLzAxXC80MDAwMzA0XC9wcm9qZWN0c1wvNFwvYXNzZXRzXC8zOFwvMTc2XC9jZGYwZTJlOTEyNDFlYzAzZTQ1YTVhZTc4OGQ0ZDk0MS0xNjA1MjIyMzU4LnBuZyJ9:dfinity:9Q2_9PEsbPqdJNAQ08DAwqOenwIo7A8_tCN4PSSWkAM?width=2400" alt="Logo" width="150" height="150" />
