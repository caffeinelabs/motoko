---
title: "Shared types"
description: "All Motoko types are divided into sets."
sidebar:
  order: 2
---

All Motoko types are divided into sets. The smallest is the set of shared types. Shared types are part of the larger set of [stable types](./stable-types.md).  

A shared type's value can be easily exchanged with other actors. To prevent issues associated with sharing mutable state across actors, **all shared types are immutable**. This immutability allows values to be transmitted safely by copying data, avoiding the complexity and risks of sharing stateful or mutable objects.

Motoko's shared types also have a natural translation to Candid types.

Operationally, you can think of shared types as the subset of types that can be serialized to Candid.

In Motoko, shared functions are used to transmit values between actors. Unlike other functions, their arguments and results are restricted to shared types. This restriction allows the arguments and results to be translated and transmitted as Candid.

These functions are shared because they are also values of shared types and can be communicated to other actors.

## Importance of shared types

Shareability is essential for several reasons:

- Shared types ensure data can be safely serialized and deserialized across network boundaries.
- Restricting sharing to immutable data prevents synchronization issues between canisters.
- Shared types map directly to [Candid](https://docs.internetcomputer.org/guides/canister-calls/candid), enabling interaction between canisters written in different languages.
- Web and mobile frontends communicate with canisters using shared types.

## Common shared types

### Primitive types

Most [primitive types](./primitive-types.md) are shared by default.

```motoko no-repl
// Numbers, text, and booleans are shared
let number : Nat = 42;
let message : Text = "Hello IC";
let flag : Bool = true;
```

### Immutable collections

Collections that cannot be modified after creation are shared, including [immutable arrays](./immutable-arrays.md) and [tuples](./tuples.md).

```motoko no-repl
// Immutable arrays are shared
let names : [Text] = ["Motoko", "Ghost", "Astronaut"];

// Tuples of shared types are shared
let person : (Text, Nat) = ("Motoko", 25);
```

### Records with immutable fields

Objects with immutable fields containing shared types are shared, including [records](./records.md).

```motoko no-repl
// Records with immutable fields are shared
let user = {
    id = "usr123";
    name = "Motoko";
    age = 30;
};
```

### Variants with shared type tags

[Variant types](./variants.md) are shared when their tags contain shared types.

```motoko no-repl
// Variant types with shared tags are shared
type Result = {
    #ok : Nat;
    #error : Text;
};

let success : Result = #ok(200);
let failure : Result = #error("Operation failed");
```

### Option types

[Option types](./options.md) are shared when they contain shared types.

```motoko no-repl
// Option types with shared inner types are shared
let maybeGreeting : ?Text = ?"Hello";
let nothing : ?Nat = null;
```

### Actor references

References to [actors](../actors/actors-async.md) are shared, allowing [canisters](https://docs.internetcomputer.org/concepts/canisters) to call each other.

```motoko no-repl
// Actor types are shared
type CounterActor = actor {
    increment : shared () -> async Nat;
    getValue : shared query () -> async Nat;
};
```

### Shared functions

[Function types](./function-types.md) marked as `shared` are sharable.

```motoko no-repl
// Shared function types are shared
type Callback = shared (Nat) -> async ();
```

## Non-shared types

Certain types cannot be shared between [canisters](https://docs.internetcomputer.org/concepts/canisters).

### Mutable collections

```motoko no-repl
// Mutable arrays are NOT shared
let mutableArray : [var Nat] = [var 1, 2, 3];
```

### Objects with mutable fields

```motoko no-repl
// Objects with mutable fields are NOT shared
let mutableUser = {
    var name = "Motoko";
    var age = 30;
};
```

### Error types

```motoko no-repl
// The Error type is NOT shared
let err : Error = Error.reject("Something went wrong");
```

### Objects containing non-shared types

```motoko no-repl
// Objects containing non-shared types are NOT shared
let complex = {
    name = "Document";
    contents = [var 1, 2, 3];  // Contains a mutable array
};
```

