---
title: "Caller identification"
description: "On ICP, every user and canister has a unique principal identifier."
sidebar:
  order: 3
---

On ICP, every user and canister has a unique **principal** identifier. When a canister receives a message, such as a function call, the principal identifying the caller of the function is included in the message.

The caller’s principal is provided by the network, not the user or canister. A principal cannot be forged or spoofed by a malicious user.

Motoko’s `shared` keyword is used to declare a shared function. Shared functions support a simple form of caller identification that can be used to inspect incoming messages and implement access control based on the caller's principal. The shared function can also declare an optional parameter of type `{caller : Principal}`.

For example, when a shared function is called by a user or another canister, the caller's principal can be captured with `msg.caller`:

```motoko no-repl
shared(msg) func inc() : async () {
  // ... msg.caller ...
}
```

In this example, the shared function `inc()` specifies a `msg` parameter, a record, and the `msg.caller` accesses the principal field of `msg`.

To access the caller of an actor class constructor, you use the same syntax on the actor class declaration:

```motoko no-repl
shared(msg) persistent actor class Counter(init : Nat) {
  // ... msg.caller ...
}
```

## Adding access control

Access control can be added to an application by recording a caller's principal and then introducing control rules, such as a list of allowed principals that are permitted to call the function. If the caller's principal is not within the list, their call to the function will be rejected.

For example, if you have an actor called `Counter`, you can record the principal that installed the actor by binding it to an `owner` variable. Then, you can implement a check that verifies that the caller of each method the actor exposes is equal to the principal stored in `owner`:

```motoko file=<motokoExamples>/Counters-caller.mo
```

In this example, the `assert (owner == msg.caller)` expression causes the functions `inc()` and `bump()` to trap if the call is unauthorized, preventing any modification of the `count` variable. However, the `read()` function permits any caller.

The argument to `shared` is just a pattern. You can rewrite the above to use pattern matching:

```motoko file=<motokoExamples>/Counters-caller-pat.mo
```

:::note

Simple actor declarations do not let you access their installer. If you need access to the installer of an actor, rewrite the actor declaration as a zero-argument actor class instead.

```motoko no-repl
shared(msg) actor class InstallerAware() {
  let installer = msg.caller; // This is the principal of the installer

  public func whoInstalled() : async Principal {
    installer
  };
}
```

:::


## Recording principals

Principals support equality, ordering, and hashing, so you can efficiently store principals in containers for functions such as maintaining an allow or deny list. More operations on principals are available in the [principal](https://mops.one/core/docs/Principal) core module.

The data type of `Principal` in Motoko supports equality, ordering, and hashing. `Principal`s both sharable and stable, meaning you can compare them for equality directly.

Below is an example of how you can record a function's caller `Principal`s in a set and check if a caller is already in the set:

```motoko file=<motokoExamples>/RecordPrincipals.mo
```


