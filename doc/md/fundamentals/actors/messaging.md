---
title: "Messaging"
description: "ICP enforces rules on when and how canisters communicate."
sidebar:
  order: 5
---

ICP enforces rules on when and how [canisters](https://docs.internetcomputer.org/concepts/canisters) communicate. Motoko includes static (compile-time) messaging restrictions to help prevent certain execution errors.

For example, a canister cannot send messages during installation, which helps avoid errors during deployment. Query functions cannot send messages either, because they run locally and do not trigger updates. Additionally, shared functions cannot be called in a synchronous context since shared calls require asynchronous execution.

Only async contexts support [error handling](../error-handling.md) with `try/catch` because messaging errors only occur asynchronously.

In Motoko, an expression is considered to be in an async context if it appears inside an `async` function. Query functions are read-only, so they do not create an async context and therefore cannot use `await` or send messages.

```motoko no-repl
persistent actor Counter {
    var count : Nat = 0;

    public func increment() : async () {
        count += 1;
    };

    public query func getCount() : async Nat {
        return count;  // Allowed: No state change
    };

    public func invalidCall() : Nat {
        return await getCount();  // Error: Query function cannot be awaited in a sync function
    };
};
```

## Inter-canister calls

One of the key features of ICP is the ability for canisters to invoke functions in other canisters. This capability, known as inter-canister calls, allows canisters to interact with each other.

There are different methods for making inter-canister calls. The primary and recommended method is direct import, which is used when the target canister is part of the same project and explicitly imported. For example, `import Subscriber "canister:subscriber";` allows direct access to that canister’s functions.

The second method uses actor type annotations and should be rarely used. This approach applies when calling an external canister that is part of the project but deployed separately. An example is `let sub = actor(canisterId) : actor { notify : Text -> async (); };`, which creates a typed reference to the external canister.

The third method involves dynamic calls. This is useful when calling unknown functions or when the arguments are dynamic. For example, `await IC.call(canisterId, methodName, encodedArgs);` lets you make flexible calls without static typing.

### Canister imports

When a canister exists in the project directory, it can be imported using the `import` statement. This ensures strong typing and allows safe function calls.

In this example, a publisher canister maintains a list of subscribers and sends notifications when an event occurs. Each subscriber canister receives and processes notifications.

#### Publisher

```motoko no-repl
import Subscriber "canister:subscriber";
import Array "mo:core/Array";

actor Publisher {
    stable var subscribers : [Subscriber.Subscriber] = [];

    public shared func subscribe(subscriber : Subscriber.Subscriber) : async () {
        if (Array.find<Subscriber.Subscriber>(subscribers, func(s) { s == subscriber }) == null) {
            let newSubscribers = Array.tabulate<Subscriber.Subscriber>(
                subscribers.size() + 1,
                func(i) { if (i < subscribers.size()) subscribers[i] else subscriber }
            );
            subscribers := newSubscribers;
        };
    };

    public shared func publish(message : Text) : async () {
        for (sub in subscribers) {
            ignore await sub.notify(message);
        };
    };
};

```

#### Subscriber

```motoko no-repl
import Debug "mo:core/Debug";

actor Subscriber {
    public shared func notify(message : Text) : async () {
        Debug.print("Received message: " # message);
    };
};

```