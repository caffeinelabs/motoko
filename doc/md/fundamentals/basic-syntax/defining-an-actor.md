---
title: "Defining an actor"
description: "In Motoko, an actor is a computational process with its own state and behavior."
sidebar:
  order: 1
---

In Motoko, an **actor** is a computational process with its own [state](../actors/state.md) and behavior. Actors are declared with the `actor` keyword.

Unlike traditional functions or objects in other programming languages, actors operate independently and communicate via [asynchronous](../actors/actors-async.md#async--await) messaging. Each actor maintains its own message queue, enabling concurrent execution.

An actor's state is defined by its private variables, while its behavior is defined by the public functions it exposes to other actors.

You should define an actor when you want to encapsulate state and expose a public API that can be accessed asynchronously by other actors, canisters, or external clients.

More specifically, define an actor when:

- You are building a canister smart contract that maintains private state and exposes public functions.
- You want to create an application that runs on the Internet Computer and is accessible by users or other canisters.
- You want to take advantage of the actor model's benefits, such as memory isolation, single-threaded execution for update calls (avoiding race conditions), and asynchronous communication.

In Motoko, actors are defined at the top level of a source file using the `actor` keyword. Public functions within an actor return their results in `async` types (otherwise known as futures) to support asynchronous calls from remote callers.

An actor definition is required for a Motoko program to be deployed as a canister on ICP.

Each actor maintains separate queues of incoming messages, one per sender. Messages are processed in order, ensuring that one message cannot interfere with another. This protects the actor's state from concurrent modification.

Since actors process messages independently, multiple actors can handle messages in parallel, enabling concurrent execution across actors.


```motoko no-repl
// Declares an actor named Main.
persistent actor Main {
  // Define a private variable called 'count' to track the number of greetings.
  var count : Nat = 0;

  // Define a public function that asynchronously returns a greeting
  // and increments the counter.
  public func greet(name : Text) : async Text {
    count += 1;
    "Hello, " # name # "! You are visitor number " # debug_show(count);
  };

  // Define a publicly called function to
  // return the current value of 'count' separately.
  public query func readCount() : async Nat {
    count
  };
};
```

:::note
This code defines an actor that can be deployed on ICP.
The actor is declared as `persistent` so that its state, `count`, will be preserved
when the actor is upgraded.
Learn more about [persistence](../actors/data-persistence.md).
:::

Another actor can call `Main.greet()` with an argument and await the result:

```motoko no-repl
await Main.greet("Programmer");
```

A Motoko actor always presents its interface as a suite of named [functions](./functions.md) (also called methods) with defined argument and return types. When Motoko code is compiled, this interface is automatically translated to [Candid](https://docs.internetcomputer.org/guides/canister-calls/candid), an interface description language. The Candid description can be consumed by other canisters, including canisters written in another language such as Rust.

The above example's corresponding Candid interface can be found below.

```did
service : {
  greet : (text) -> (text);
  readCount : () -> (nat) query;
}
```


## Resources

- [Actors](../actors/actors-async.md)

