---
title: "Mutable state"
description: "In Motoko, each actor can use internal mutable state but cannot share it directly with other actors."
sidebar:
  order: 2
---

In Motoko, each actor can use internal mutable state but cannot share it directly with other actors. Immutable data, however, can be shared among actors and accessed via their external entry points, which act as shareable functions.

Mutable state is private to the actor that owns it and can be modified internally. In contrast, immutable values cannot be changed after creation and can be safely shared between actors.

For example, the following actor maintains a private mutable counter that can only be modified through its public API, in particular this ensures the counter can never decrease:

```motoko no-repl
actor Counter {
    stable var count : Nat = 0; // Private mutable state

    public shared func increment() : async Nat {
        count += 1;
        return count;
    };

    public query func getCount() : async Nat {
        return count;
    };
};
```

Since `count` is mutable, it can be modified internally but cannot be accessed directly from outside the actor. Instead, other actors must use its public interface to retrieve or update its value.

