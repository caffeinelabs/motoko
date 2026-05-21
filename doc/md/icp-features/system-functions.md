---
title: "System functions"
description: "ICP supports five system functions that canisters can call to interact with the ICP runtime environment:"
sidebar:
  order: 6
---

ICP supports five system functions that canisters can call to interact with the ICP runtime environment:

- [`timer`](#timer)
- [`preupgrade`](#preupgrade)
- [`postupgrade`](#postupgrade)
- [`lowmemory`](#lowmemory)
- [`inspect`](#inspect)
- [`heartbeat`](#heartbeat)
  

Declaring any other system function will result in an error. Canisters can use these functions to efficiently manage state transitions, automate tasks, or handle system-level operations. 

## `timer()`

The [`timer()` system function](https://docs.internetcomputer.org/guides/backends/timers#recurring-timers) lets canisters schedule a task to execute after a specified delay. To make the timer repeat, the function must explicitly call `setGlobalTimer()` within its body to reset the timer. It accepts a single argument to set the global timer and returns `async ()`.

Unlike `heartbeat()`, which runs automatically every subnet round, `timer()` requires manual rescheduling after each execution. This design gives canisters precise control over whether the timer runs once or continuously, depending on if and when `setGlobalTimer()` is called again.

In the following example, `timer()` runs once immediately after deployment, then stops.

```motoko no-repl
import Debug "mo:core/Debug";

system func timer(setGlobalTimer : Nat64 -> ()) : async () {
  Debug.print("Timer triggered!");
  // No call to setGlobalTimer() → the timer does not repeat.
}
```

To run the timer every 20 seconds, it must be explicitly rescheduled.

```motoko no-repl
import Time "mo:core/Time";
import Debug "mo:core/Debug";

system func timer(setGlobalTimer : Nat64 -> ()) : async () {
  let next = Nat64.fromIntWrap(Time.now()) + 20_000_000_000; // 20 seconds
  setGlobalTimer(next); // Reschedule for next execution
  Debug.print("Repeating Timer Triggered!");
}
```

## `preupgrade()`

The `preupgrade()` system function is invoked immediately before a canister upgrade. It runs before the new Wasm module is installed, giving the current version one last chance to act. The function takes no arguments and must have type `() -> ()`.

```motoko no-repl
persistent actor MyCanister {
  system func preupgrade() {
    // Runs before the upgrade installs the new Wasm.
  }
}
```

:::danger
If `preupgrade` traps, runs out of cycles, or hits any other IC computing limit, **the upgrade fails and the canister cannot be upgraded going forward** — it is stuck on the current version. Use of this hook is discouraged.
:::

With orthogonal persistence, `mo:core` data structures persist across upgrades automatically and this hook is rarely needed. For the (legacy) save-into-stable-storage pattern and the migration alternatives that replace it, see [Data persistence](../fundamentals/actors/data-persistence.md).

## `postupgrade()`

The `postupgrade()` system function runs immediately after an upgrade installs the new Wasm. The function takes no arguments and must have type `() -> ()`.

```motoko no-repl
persistent actor MyCanister {
  system func postupgrade() {
    // Runs after the upgrade installs the new Wasm.
  }
}
```

`postupgrade` is rarely required: the same effect can usually be achieved with actor initialization expressions (`let` bindings and statements at the top of the actor body), which run on every install and upgrade. See [Data persistence](../fundamentals/actors/data-persistence.md) for the recommended patterns.

## `lowmemory()`

The IC allows to implement a low memory hook, which is a warning trigger when main memory is becoming scarce.

For this purpose, a Motoko actor or actor class instance can implement the system function `lowmemory()`. This system function is scheduled when canister's free main memory space has fallen below the defined threshold `wasm_memory_threshold`, that is is part of the canister settings. In Motoko, `lowmemory()` implements the `canister_on_low_wasm_memory` hook defined in the IC specification.

Example of using the low memory hook:
```
actor {
    system func lowmemory() : async* () {
        Debug.print("Low memory!");
    }
}
```

The following properties apply to the low memory hook:
* The execution of `lowmemory` happens with a certain delay, as it is scheduled as a separate asynchronous message that runs after the message in which the threshold was crossed.
* Once executed, `lowmemory` is only triggered again when the main memory free space first exceeds and then falls below the threshold.
* Traps or unhandled errors in `lowmemory` are ignored. Traps only revert the changes done in `lowmemory`.
* Due to its `async*` return type, the `lowmemory` function may send further messages and `await` results.

## `inspect()`

The [`inspect()` system function](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface#system-api-inspect-message) allows a canister to inspect ingress messages before execution, determining whether to accept or reject them. The function receives a record of message attributes, including the caller’s principal, the raw argument `Blob`, and a variant identifying the target function.
 
It returns a `Bool`, where `true` permits execution and `false` rejects the message. Similar to a [query](https://docs.internetcomputer.org/references/message-execution-properties), any side effects are discarded. If `inspect()` traps, it is equivalent to returning `false`. Unlike other system functions, the argument type of `inspect()` depends on the actor's exposed interface, meaning it can selectively handle different methods or ignore unnecessary fields. 

However, `inspect()` should not be used for definitive access control because it runs on a single replica without going through consensus, making it susceptible to boundary node spoofing. Additionally, `inspect()` only applies to [ingress messages](https://docs.internetcomputer.org/references/message-execution-properties), not [inter-canister calls](https://docs.internetcomputer.org/references/message-execution-properties), meaning secure access control must still be enforced within shared functions.

The following actor defines an inspect function that blocks anonymous callers, limits message size, and rejects specific argument values.

```motoko no-repl
import Principal "mo:core/Principal";

persistent actor Counter {
  
  var c = 0;

  public func inc() : async () { c += 1 };
  public func set(n : Nat) : async () { c := n };
  public query func read() : async Nat { c };
  public func reset() : () { c := 0 }; // One way function

  system func inspect(
    {
      caller : Principal;
      arg : Blob;
      msg : {
        #inc : () -> ();
        #set : () -> Nat;
        #read : () -> ();
        #reset : () -> ();
      }
    }) : Bool {

    if (Principal.isAnonymous(caller)) return false; // Reject anonymous calls
    if (arg.size() > 512) return false; // Reject messages larger than 512 bytes

    switch (msg) {
      case (#inc _) { true };   // Allow increment
      case (#set n) { n() != 13 }; // Reject setting counter to 13
      case (#read _) { true };  // Allow reading the counter
      case (#reset _) { false }; // Reject reset calls
    }
  }
}
```

## `heartbeat()`

:::caution
Heartbeats are computationally expensive for both the network and user, and instead you should use a timer if possible.
:::

Canisters can opt to receive [heartbeat messages](https://docs.internetcomputer.org/guides/backends/timers#heartbeats) by exposing a `canister_heartbeat` function. In Motoko, this is achieved by declaring the system function `heartbeat`, which takes no arguments and returns an asynchronous unit type (`async ()`).

Since `heartbeat()` is async, it can invoke other asynchronous functions and await their results. This function executes on every **subnet heartbeat**, enabling periodic task execution without requiring external triggers. Since subnet heartbeats operate at the protocol level, their timing is not precise and depends on network conditions and execution load. As a result, using heartbeats for high-frequency or time-sensitive operations should be done cautiously, as there is no guarantee of real-time execution.

Every async call in Motoko causes a context switch, which means the actual execution of the heartbeat function may be delayed relative to when the subnet triggers it. The function’s result is ignored, so any errors or traps during execution do not impact future heartbeat calls.

If a canister exports a function named `canister_heartbeat`, it must have the type `() -> ()`, ensuring it adheres to the expected [system function signature](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface#heartbeat).

Heartbeats should be considered deprecated as they have been superseded by timers.
