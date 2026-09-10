---
title: "Hello, world!"
description: "\"Hello, world!\" is a common starting point used to showcase a programming language's basic syntax."
sidebar:
  order: 1
---

"Hello, world!" is a common starting point used to showcase a programming language's basic syntax.

Below is an example of "Hello, world!" written in Motoko:

```motoko no-repl
// Every actor is `persistent` by default: its fields persist across canister upgrades.
actor HelloWorld {
  // We store the greeting in a variable that persists over canister upgrades.
  var greeting : Text = "Hello, ";

  // This update method modifies the greeting prefix.
  public func setGreeting(prefix : Text) : async () {
    greeting := prefix;
  };

  // This query method returns the currently persisted greeting with the given name.
  public query func greet(name : Text) : async Text {
    return greeting # name # "!";
  };
};
```

In this example:

1. The code begins by defining an [actor](./actors/actors-async.md) named `HelloWorld`. In Motoko, an actor is an object capable of maintaining state and communicating with other entities via message passing.

2. It then declares the variable `greeting`. Like every actor field, `greeting` is persisted across canister upgrades, so the prefix the canister has been set to is never lost — even when the canister is upgraded with new code. [Read more about canister upgrades.](https://docs.internetcomputer.org/guides/canister-management/lifecycle#upgrade-a-canister)

3. An [update method](https://docs.internetcomputer.org/concepts/canisters#update-calls) named `setGreeting` is used to modify the canister’s state. This method specifically updates the value stored in `greeting`.

4. Finally, a [query method](https://docs.internetcomputer.org/concepts/canisters#query-calls) named `greet` is defined. Query methods are read-only and return information from the canister without changing its state. This method returns the current `greeting` value, followed by the input text. The method body produces a response by concatenating `"Hello, "` with the input `name`, followed by an exclamation point.

[Learn more about actors and basic syntax](./basic-syntax/defining-an-actor.md).