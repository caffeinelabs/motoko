---
title: "Class declarations"
description: "A class in Motoko serves as a blueprint for creating objects that encapsulate both state and behavior."
sidebar:
  order: 4
---

A class in Motoko serves as a blueprint for creating [objects](./object-declaration.md) that encapsulate both [state](../actors/state.md) and behavior. It defines fields to hold data and methods to operate on that data. Unlike records and plain objects, classes support constructors, allowing developers to initialize each instance with unique values at creation time.

Classes in Motoko are not the same as classes in other object oriented programming languages, but they serve the same purpose. Motoko also doesn’t have a `this` or `self` keyword because you can simply call other methods directly by name or name the entire object using an identifier of your choice.

Use a class when you need to:
- Create multiple objects with the same fields and methods.
- Encapsulate state and behavior inside an object.
- Simulate object orientated programming-like design patterns.

Classes are particularly useful when using multiple objects with similar structures but different initial states. You can think of classes as a template for creating objects. You define fields and methods, and then create new instances by calling the class like a function.

## Defining a class

A class in Motoko defines both:

- A constructor for creating instances, which includes:
  - Fields to store data.
  - Methods to perform operations on that data.
- A type that describes the interface of the instance, including its available methods.

Motoko also supports generic classes, which allow abstraction over types. This enables the creation of reusable, type-safe components that can work with a variety of data types while maintaining strong compile-time guarantees.

```motoko no-repl
import Order "mo:core/Order"; // Required to reference O.Order and its variants

// A generic comparator class for ordering elements
class Comparator<T>(compare : (T, T) -> Order.Order) {
  public func lessThan(a : T, b : T) : Bool {
    compare(a, b) == #less;
  };

  public func equal(a : T, b : T) : Bool {
    compare(a, b) == #equal;
  };

  public func greaterThan(a : T, b : T) : Bool {
    compare(a, b) == #greater;
  };
};

```

This declaration defines:

- A constructor: `Comparator : <K>((K, K) -> Order) -> Comparator<K>`.
- A generic type: `Comparator<K>` that provides comparison methods.

## Creating instances

Creating an instance is when a class constructor is used to produce a new object with its own state and behavior based on the class definition.

```motoko no-repl
class Counter(init : Nat) {
  var count : Nat = init;

  public func inc() : Nat {
    count += 1;
    count
  };
};

// Creating an instance of the class
let myCounter = Counter(0);

// Using the instance
myCounter.inc(); // returns 1
```

The class declaration syntax also lets you:
* Check that the class conforms to some interface, expressed as an object type.
* Define a self-identifier to refer to the entire object.

```motoko no-repl
type Reset = { reset : () -> () };

class Counter(init : Nat) : Reset = this {
  var count : Nat = init;

  public func inc() : Nat {
    count += 1;
    count
  };

  public func getThis() : Counter { this };

  public func reset() { count := 0 };
};
```

This extended `Counter` class checks that the implementation matches interface `Reset` and names this parameter `self`.

## Actor classes

Actor classes enable you to create networks of actors programmatically. These classes must be defined in separate source files.

```motoko no-repl title="Counter.mo"
persistent actor class Counter(init : Nat) {
  var count : Nat = init;

  public func inc() : async Nat {
    count += 1;
    count;
  };

  public func get() : async Nat {
    count;
  };
}
```

This defines a `Counter` actor class with:

- A mutable variable `count`.

- A method `inc` that increases `count` and returns the new value.

- A method `get` that returns the current count.

Then, in another file you can create an instance of this actor class:


```motoko no-repl title="CallCounter.mo"
import Counter "./Counter";

persistent actor {

  public func test() : async () {
    let counter = await (with cycles = 1_000_000_000_000) Counter.Counter(0);

    let newCount = await counter.inc();

    Debug.print("Count is now: " # Nat.toText(newCount));

    let currentCount = await counter.get();
    Debug.print("Current count: " # Nat.toText(currentCount));
  }
}
```

Unlike object classes, actor class constructors are asynchronous. The constructor returns a future that contains the instance. You obtain the instance itself using `await`.

:::note

On ICP, calls to a class constructor must be provisioned with cycles to pay for the creation of a principal. See [Cycles](https://mops.one/core/docs/Cycles) for instructions on how to add cycles to a call using the imperative `Cycles.add<system>(cycles)` function.

:::

### Configuring and managing actor class instances

On ICP, the primary constructor of an imported actor class always creates a new principal and installs a fresh instance of the class as the code for that principal.

To offer more control over actor class installation, Motoko provides each imported actor class with a secondary constructor. This constructor takes an additional first argument that specifies the desired installation mode.

This advanced constructor is only available via special `system` syntax, highlighting its low-level nature.

Using this `system` syntax, developers can:

- Specify initial canister settings (e.g., an array of controllers).
- Manually install, upgrade, or reinstall canisters.
- Access lower-level canister management features provided by ICP.

[Learn more about actor class management](../../language-manual.md#actor-class-management).

