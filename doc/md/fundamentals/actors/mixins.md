---
title: "Mixins"
description: "Using mixins to compose reusable actor components in Motoko."
sidebar:
  order: 7
---

Mixins allow defining parts of actors in separate re-usable files that can be combined/included into a complete actor.

Mixins are defined using the `mixin` keyword, and accept a parameter list like functions. They can define fields, types, and (public) methods just like an actor.

Inside an actor a mixin can then be included with the `include` keyword while passing a parameter list. The definitions of the mixin get added to the including actor. The mixin cannot refer to the fields of the actor unless explicitly passed via the parameter list. Fields in the actor and its included mixins must not overlap.

As an example we'll consider a logging mixin that makes a `logError` function available inside the actor and exposes a `collectLogs` endpoint from the canister. It also allows prefixing any collected log messages.

```motoko no-repl
// Logger.mo
import List "mo:core/List";

mixin(prefix : Text) {
  type Log = { severity : Text; msg : Text };
  let logs = List.empty<Log>();

  func logError(msg : Text) {
    logs.add({ severity = "Error"; msg = prefix # msg })
  };

  public shared func collectLogs() : [Log] {
    let res = logs.toArray();
    logs.clear();
    res
  };
}
```

We can then use it inside our main actor

```motoko no-repl
// main.mo
import Logger "Logger";
actor {
  include Logger("[Greeter] ");

  public shared func greet(name : Text) : async Text {
    if (name == "") {
      logError("Saw an empty name");
    };
    "Hello, " # name # "!"
  };
};
```

The resulting actor will collect error logs when greeting empty names and return logs like:
`{ severity = "Error"; msg = "[Greeter] Saw an empty name" }`
