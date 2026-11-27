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
actor Counter {
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
        if Array.find<Subscriber.Subscriber>(subscribers, func(s) { s == subscriber }) == null {
            let newSubscribers = Array.tabulate<Subscriber.Subscriber>(
                subscribers.size() + 1,
                func(i) { if i < subscribers.size() { subscribers[i] } else { subscriber } }
            );
            subscribers := newSubscribers;
        };
    };

    public shared func publish(message : Text) : async () {
        for sub in subscribers {
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

## Dynamic calls and function references: pitfalls

Dynamic calls (`IC.call` from `mo:core/InternetComputer`) and calls through shared function values that a caller passed in (callbacks) are sent *by your canister*. The callee sees your canister's principal as the caller, whoever chose the target. Keep the following in mind.

### Your canister acts on the caller's behalf

A canister that calls a caller-chosen target, method, or function value is a potential *confused deputy*. If the target is your canister itself, the call arrives with `caller == Principal.fromActor(this)`. If your canister is one of its own controllers, `Principal.isController(caller)` holds as well. Any method that grants privileges on these checks can then be invoked by anyone who can reach the forwarding code.

```motoko no-repl
import IC "mo:core/InternetComputer";
import Principal "mo:core/Principal";

actor Executor {

  // Unsafe: any caller can make this canister call itself.
  public func execute(target : Principal, method : Text, args : Blob) : async Blob {
    await IC.call(target, method, args)
  };

  // Safer: refuse this canister as target (or use an allow list of targets and methods).
  public func executeChecked(target : Principal, method : Text, args : Blob) : async Blob {
    assert target != Principal.fromActor(Executor);
    await IC.call(target, method, args)
  };
};
```

A shared function value is opaque: Motoko offers no way to inspect the principal or the method name it refers to, so it cannot be filtered like the explicit target above. Only call function values received from callers you trust, or do not grant privileges based on `caller == Principal.fromActor(this)` or on your canister being its own controller.

### Arguments and replies are not checked

`IC.call` takes and returns raw Candid bytes. Nothing checks that `args` matches the callee's method type, or that the reply has the type you expect. Decode replies with `from_candid` and handle the `null` that it returns on a mismatch, rather than assuming success.

### Calling a query method makes an update call

A dynamic call is always an update call, even if the target method is a query. It is executed with consensus and costs cycles accordingly.

### State can change while you await

Other messages may run between sending a call and receiving its reply. Re-check any state that your code relied on before the `await`, as for any other inter-canister call.

### Parentheticals apply

`(with cycles = …; timeout = …) IC.call(…)` attaches cycles to the call or makes it a best-effort call with a timeout, just like for other calls.
