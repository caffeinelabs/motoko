---
title: "Function types"
description: "Functions are reusable chunks of code that perform a specific task."
sidebar:
  order: 3
---

Functions are reusable chunks of code that perform a specific task. A function is defined with a name and optional parameters, then returns a defined result. A function can also specify a return type for the value it produces.

In the process of producing a result, the execution of the function can have other side effects, like modifying state, printing to the log, or sending messages to other canisters.

Functions can be synchronous or asynchronous. A synchronous function blocks the caller until it returns with a result, just like functions in most traditional programming languages.

An asynchronous function returns immediately, providing a **future** value as a placeholder for its result. The caller can await the future later to retrieve the result or ignore it and continue with other tasks.

Motoko offers different types of functions, each with distinct capabilities:

- **Local functions**, declared using the `func` keyword, are typically synchronous but can be asynchronous if their body contains an `async` expression. A local function is only available within the actor that defines it; it cannot be called from another actor or sent to another actor in a message.

- **Shared functions**, declared using the `shared`, `shared query`, or `shared composite query` keywords, are asynchronous by nature. Calling a shared function sends a message to another actor. The caller is typically another Motoko actor, a [canister](https://docs.internetcomputer.org/concepts/canisters), or an [agent](https://docs.internetcomputer.org/guides/canister-calls/calling-from-clients).

An actor's shared functions are always called as the result of the actor receiving some message. Shared functions that return a result have `async` return types.

Since actors can only have `shared` functions as public members, the `shared` keyword  is optional and can be omitted from public actor functions.

Motoko provides different types of functions based on where in the program they execute and how they interact with the system. Understanding these distinctions is essential when designing efficient and scalable canister logic.

## Function keywords

| Keyword  | Function  |
|-------------|--------------|
| `shared`    | Used to enable async communication between actors. Exposes the caller’s identity. |
| `async`     | Runs the function [asynchronously](../actors/actors-async.md#async--await) and returns its result in a future. |
| `query`     | Optimized for reading data but cannot modify [state](../actors/state.md). |

## Function comparison

| Function type                | Mutates [state](../actors/state.md) | Calls updates | Calls queries | Asynchronous | External calls |
|------------------------------|---------------|------------------|------------------|---------------|---------------|
| Local   (synchronous)                 |  Yes            | No              | No             | No    | No            |
| Local   (asynchronous)                | Yes            | Yes              | Yes              | Yes   |  No            |
| Shared (update)         | Yes           | Yes              | Yes              | Yes           | Yes           |
| Shared query              | No            | No               | No              | Yes           | Yes           |
| Shared composite query   | No            | No               | Yes              | Yes           | Yes           |

## Local functions {#pure-functions}

Local functions run within the canister's [actor](../actors/actors-async.md). They cannot call other [canisters](https://docs.internetcomputer.org/concepts/canisters). Local functions are cheap to call and execute synchronously.

```motoko no-repl
persistent actor CommonDivisor{
  func gcd(a : Nat, b : Nat) : Nat {
    var x = a;
    var y = b;

    while (y != 0) {
      let temp = y;
      y := x % y;
      x := temp;
    };

    x  // Return value is the GCD
  };
let greatestCommonDivisor : Nat = gcd(108, 54); // Synchronous execution
};
```

The type of `gcd` is `(Nat, Nat) -> Nat` indicating that it expects a pair of naturals as the input argument and returns a natural as a result.

**Example use case:** Local computations that do not require communication with other actors or canisters.

## Generic functions

Generic functions allow the use of type parameters, making them more flexible for using different data types.

```motoko no-repl
func swap<T, U>(t : T, u : U) : (U, T) {
  (u, t)
};

swap<Nat, Text>(42, "ICP"); // ("ICP", 42)
```

The type of `swap` is `<T, U> (T, U) -> (U, T)`, indicating it can accept any two types `T` and `U`, a value of tuple type `(T, U)` and return a tuple of type `(U, T)`, with the elements swapped.

Type arguments can be omitted from calls when the compiler can infer them from the arguments and context, allowing the simpler code:

```motoko no-repl
let result = swap(2021, "Motoko"); // Inferred as <Nat, Text>
```

## Local asynchronous functions

Local function that have an `async` or `async*` return type are asynchronous and can interact with other canisters by calling shared functions.

They are useful for defining asynchronous logic used in the implementation of public `shared` functions.

```motoko no-repl
import Time "mo:core/Time";
import Logger "canister:Logger";

persistent actor {
  private func log(msg : Text) : async () {
    Logger.log(Time.now() + msg); // sends a message
  };
  public shared func doStuff() : async () {
    await log("doingStuff");
  }
}
```

A more efficient variation is to use `async*` and `await*` , which avoids the overhead of using ordinary `await` just to call a local function:

```motoko no-repl
import Time "mo:core/Time"
import Logger "canister:Logger";

persistent actor {
  private func log(msg : Text) : async* () {
    await Logger.log(Time.now() + msg); // sends a message
  };

  public shared func doStuff() : async () {
    await* log("doingStuff");
  }
}
```

## Shared functions

The public functions of an actor determine its external interface. All public functions in an actor must be shared and can be either `shared`, `shared query` or `shared composite query` functions. Private functions cannot be `shared`.
Since an actor's public functions must be shared, the `shared` keyword is optional and can be omitted.

`shared` functions permanently update the state of an actor, while `query` and `composite` `query` functions are only executed for their result.

Although queries can temporarily alter the state of an actor, these changes are not permanent and are never visible to other callers. It's as if each query operates on a copy of the actor, which is discarded when the query returns.

When called from a front-end, `query` functions generally have much lower latency than equivalent shared functions. This is because shared functions require the protocol to reach consensus on the state changes and results, whereas query functions do not.

```motoko no-repl
persistent actor Account {
  var balance = 0;

  public shared func deposit(amount : Nat) : async Nat {
    balance += amount;
    balance
  }
}
```

Omitting the shared keyword, we can also write:

```motoko no-repl
persistent actor Account {
  var balance = 0;

  public func deposit(amount : Nat) : async Nat {
    balance += amount;
    balance
  }
}
```

The deposit function has type `: shared Nat -> async Nat`.
Consider this code:

```motoko no-repl
let b1 = await Account.deposit(50);
let b2 = await Account.deposit(50);
(b1,b2)
```
The first class to `Account.balance(50)` increments `balance` from `0` to `50`, returning `50`.
The second call increments the balance from `50` to `100`, returning `100`.

Since `Account.deposit` is asynchronous, its results are returned in futures of type `async Nat`. Calling `await` on each future extracts the results of the calls when they become available (so `b1` is `50` and `b2` is `100`).

**Example use case**: Transactions, user [state](../actors/state.md) updates, or anything that modifies persistent data.

### One-way functions

An update function can, but need not, return a future. An update function can just return `()` to indicate that it can be called for its side effect, but its result cannot be awaited.
Such a function is called a _one-way_ (or _fire-and-forget_) function.

An example of this might be a variant of `Account.deposit`, `Account.credit`, that merely updates the balance without returning its new value:

```motoko no-repl
persistent actor Account {
  var balance = 0;
  public func credit(amount : Nat) : () {
    balance += amount;
  }
}
```

Calling `Account.credit(100)` updates the balance by `100`;

```motoko no-repl
Account.credit(100);
```

Again, the shared keyword is optional. Note that `Account.credit(100` just returns control; it doesn't return a future that you can await. The call is executed asynchronously.


**Example use case**: Log messages, asynchronous notifications, and messages that don't require explicit acknowledgements or return values.

## Query functions

[Query](https://docs.internetcomputer.org/concepts/canisters#query-calls) functions are designed for retrieving data. They cannot permanently update [state](../actors/state.md) and execute faster than [update](https://docs.internetcomputer.org/concepts/canisters#update-calls) functions because they do not go through consensus. Query functions are identified with the `query` keyword. Any function without the `query` keyword is an [update](https://docs.internetcomputer.org/concepts/canisters#update-calls) function.

```motoko no-repl
  public query func greet(name : Text) : async Text {
    return greeting # name # "!";
  };
```


```motoko no-repl
persistent actor Account {
  var balance  = 0;
  public shared query func getBalance() : async Nat {
     balance
  };
}
```

Again, you can omit the shared keyword:

```motoko no-repl
persistent actor Account {
  var balance = 0;

  public query func getBalance() : async Nat {
    balance
  };
}
```

The `getBalance` function has function type `shared query () -> async Nat`.

**Example use case:** Fetching data quickly without modifying the canister [state](../actors/state.md).

### Composite queries

[Composite queries](https://docs.internetcomputer.org/concepts/canisters#composite-queries) chain multiple query calls together within the same function.

A good example of a composite query might be a bank that holds references to its individual accounts, implemented as separate actors, and provides a composite query that sums the deposits in all its accounts:

```motoko no-repl
persistent actor Bank {
  type Account =
    actor { getBalance() : query () -> async Nat };

  var accounts : [Account] = []

  public shared composite query func getDeposits() : async Nat {
    var deposits = 0;
    for (account in accounts.values()) {
      deposits += await account.getBalance();
    };
  deposits;
  };
}
```

Again, the shared keyword is redundant and can be omitted:

```motoko no-repl
persistent actor Bank {
    // ... code omitted ...
    public composite query func getDeposits() : async Nat {
    var deposits = 0;
    for (account in accounts.values()) {
       deposits += await account.getBalance()
    };
    deposits;
  };
};
```

The type of `getDeposits` is `shared composite query () -> Nat`.

## Passing arguments to functions

An argument is a value passed to a function when called. Arguments enable functions to process incoming data during execution.

### Single argument

A function can take a single argument of a specific type.

```motoko no-repl
  public func increment(amount : Nat) : async Nat {
    count += amount;
    count;
  }
```

### Multiple arguments and returns

Functions can accept multiple arguments and return multiple results by enclosing them in parentheses separated by commas.

```motoko no-repl
  func divRem(x : Nat, y : Nat) : (Nat, Nat) {
    (x / y, x % y)
  }
```

### Using a record as an argument

Multiple values can be passed as a single argument by encapsulating them within a [record](./records.md) type.

```motoko no-repl
  func userName(user: { name : Text; age : Nat }) : Text {
    user.name
  }
```

### Using an array as an argument

A collection of values can be passed as a single array argument.

```motoko no-repl
  public func sum(numbers : [Nat]) : async Nat {
    var total : Nat = 0;
    for (num in numbers.vals()) { total += num };
    total;
  }
```

