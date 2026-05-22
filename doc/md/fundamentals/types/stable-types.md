---
title: "Stable types"
description: "Stable types include all shared types and represent the kinds of values that can be stored in the stable declarations of a Motoko actor."
sidebar:
  order: 13
---

**Stable types** include all [shared types](./shared-types.md) and represent the kinds of values that can be stored in the `stable` declarations of a Motoko actor.
Storing a value in a `stable` declaration ensures that it persists across canister upgrades. This enables state preservation without the need for an external file system or database.

The set of stable types defines the kinds of values that can be transferred from an actor to its future upgraded versions.
Types that cannot be transferred include those whose values depend on the actor's current code, such as non-shared functions or, more generally, objects containing function members. These types are not stable because their behavior cannot be preserved independently of the code that defines them.

:::note
In Motoko, the treatment of private declarations depends on whether an actor is declared with the `persistent` keyword:

- In actors **without** the `persistent` keyword, all private declarations are considered **transient** by default, unless explicitly marked `stable`.

- In **`persistent` actors**, all private declarations (except function declarations) are considered **stable** by default, unless explicitly marked `transient`.

Stable variables must have types that belong to the set of stable types.
Transient variables are not subject to this restriction and may have any type, including non-stable types such as functions or objects with function members.

:::

To give the user more flexibility, the set of stable types is larger than the set of shared types and includes mutable types. This means that programmers can store their application state using a wide range of imperative (stateful) as well as functional (stateless) data structures.

## Stable vs shared types

While all shared types are stable, the reverse is not true. Some stable types cannot be shared across [canisters](https://docs.internetcomputer.org/concepts/canisters).

| Type                                                                                         | Stable | Shared |
|----------------------------------------------------------------------------------------------|--------|--------|
| Primitive types ([`Nat`](https://mops.one/core/docs/Nat), [`Text`](https://mops.one/core/docs/Text), [`Bool`](https://mops.one/core/docs/Bool), etc.) | Yes    | Yes    |
| Immutable arrays (`[T]`)                                                                     | Yes*   | Yes**  |
| Mutable arrays (`[var T]`)                                                                   | Yes*   | No     |
| Records with immutable fields                                                                | Yes    | Yes    |
| Records with mutable fields                                                                  | Yes    | No     |
| Option types (`?T`)                                                                          | Yes*   | Yes**  |
| Variants with stable types                                                                   | Yes    | No (if non-shared types are included) |
| Shared functions                                                                             | Yes    | Yes    |
| Actor references                                                                             | Yes    | Yes    |
| Regions                                                                                      | Yes    | No     |
| Error type (`Error`)                                                                         | No     | No     |
| Non-shared functions and futures (`async* T`)                                                | No     | No     |

\* provided `T` is stable.
\** provided `T` is shared.

:::note
In composite types like `[T]`, `[var T]`, and `?T`, the element type `T` must also be stable (for use in stable variables) or shared (for use in shared declarations). This ensures that the entire structure adheres to the requirements of stability or shareability, respectively.
Non-shared functions and futures (`async T`) and computations (`async* T`) depend on the current actor's code and execution context and therefore cannot be preserved across upgrades or transferred between actors.
:::

## Common stable types

### Primitive types

Most [primitive types](./primitive-types.md) in Motoko are stable.

```motoko no-repl
persistent actor {
  // Numbers, text, booleans and other primitive types are stable
  var counter : Nat = 0;
  var greeting : Text = "Welcome";
  var isActive : Bool = true;

};
```

### Immutable and mutable collections

Both immutable and mutable collections of stable types are stable.

```motoko no-repl
persistent actor {
  // Immutable arrays are stable
  var usernames : [Text] = ["Motoko", "Ghost"];

  // Mutable arrays are also stable (unlike shared types)
  var scores : [var Nat] = [var 100, 85, 92];
};
```

### Records with mutable or immutable fields

[Records](./records.md) that contain only stable types remain stable, regardless of whether their fields are mutable or immutable.

```motoko no-repl
persistent actor {
  // Records with immutable fields are stable
  var config = {
    appName = "My_Motoko_App";
    version = "1.0.0";
  };

  // Records with mutable fields are also stable
  var settings = {
    var darkMode = false;
    var notifications = true;
    var port = 80;
    };
};
```

### Variants with stable type tags

[Variants](./variants.md) are stable when their tags contain only stable types.

```motoko no-repl
persistent actor {
  // Variants with stable tags are stable
  type UserStatus = {
      #online;
      #offline;
      #busy : Text;
  };

  var motokoStatus : UserStatus = #online;
  var ghostStatus : UserStatus = #busy("In a meeting");
};
```

### Option types

[Option](./options.md) types are stable when they contain stable types.

```motoko no-repl
persistent actor {
  // Option types with stable inner types are stable
  var optionalDeadline : ?Nat = ?1640995200000;
  var optionalMessage : ?Text = null;
};
```

### Regions

The [`Region`](https://mops.one/core/docs/Region) type, which provides low-level memory management, is stable.

```motoko no-repl
persistent actor {
  // Regions are stable
  var storage : Region = Region.new();
  }
```

### Actor references

References to [actors](../actors/actors-async.md) are stable, allowing stable canister-to-canister interactions.

```motoko no-repl
persistent actor {
  // Actor types are stable
  type LoggerActor = actor {
      log : shared (message : Text) -> async ();
  };
  var logger : ?LoggerActor = null;
};
```

### Objects with mutable fields

Simple objects with mutable fields (but no methods) are stable. Such simple objects are the same as records.

```motoko no-repl
persistent actor {
  object user = {
      var name = "Motoko";
      var loginCount = 0;
    };
};
```

