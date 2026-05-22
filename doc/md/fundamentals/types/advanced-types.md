---
title: "Advanced types"
description: "Advanced type features enable more flexible and expressive type definitions, including structural equality, generic types, subtyping, recursive types, and ty..."
sidebar:
  order: 12
---

Advanced type features enable more flexible and expressive type definitions, including structural equality, generic types, subtyping, recursive types, and type bounds.

## Structural equality

Structural equality determines whether two values are equal based on their contents. This applies to immutable data structures, such as [records](./records.md) and [variants](./variants.md), but does not apply to mutable structures for safety reasons.

```motoko no-repl
type Point = { x : Int; y : Int };

let p1 : Point = { x = 1; y = 2 };
let p2 : Point = { x = 1; y = 2 };

p1 == p2;  // true (structural equality)
```

Even though `p1` and `p2` are distinct objects, they are considered equal because they have the same structure and values.

This remains true even if different fields are added to the point values, since the `==` on `Point`  values only considers the `x` and `y` fields and ignores other fields.

```motoko no-repl
type Point = { x : Int; y : Int };

let p1 : Point = { x = 1; y = 2; z = 3 };
let p2 : Point = { x = 1; y = 2; z = 4; c = "Red"; };

p1 == p2;  // true (structural equality at type `Point`)
```

## Generic types

Generic types are used to define type parameters that work with multiple data types, commonly used in [functions](./function-types.md), [classes](./objects-classes.md), and data structures.

```motoko no-repl
// Generic function
func identity<T>(x : T) : T {
  return x;
};

identity<Nat>(42);  // num is Nat
```

A generic class can store any type while maintaining type safety:

```motoko no-repl
class Box<T>(value : T) {
  public func open() : T { value };
};

let intBox = Box<Nat>(10);
intBox.open();
```

## Recursive types

Recursive types allow a type to refer to itself, enabling the creation of nested structures while maintaining type safety. The core package utilizes recursive types to define linked lists.

```motoko no-repl
type List = ?(Nat, List);
```

This defines a recursive type for representing a linked list of natural number. Each list is either:

- `null`, representing the empty list.

- `?(head, tail)`, where `head` is a `Nat` and `tail` is another `List`.

```motoko no-repl
?(1, ?(2, ?(3, null)))  // A list: 1 → 2 → 3
```

To generalize this structure and support values of any type, we introduce a parameterized type:

```motoko no-repl
type List<T> = ?(T, List<T>);
```

This defines a generic linked list, where `T` can be any type (`Nat`, `Text`, `Blob`, or a custom type).

### Manually reversing a linked list

Reversing a linked list involves iterating through the list and prepending each element to a new list. This approach demonstrates list traversal without using `List.reverse` library function.

Non-parameterized type:

```motoko no-repl
// Lists of naturals
type List = ?(Nat, List);

// Reverses List
func reverseNat(l : List) : List {
  var current = l;
  var rev : List = null;

  loop {
    switch (current) {
      case (?(head, tail)) {
        rev := ?(head, rev);
        current := tail;
      };
      case (null) {
        return rev;
      };
    };
  };
};
```

```motoko no-repl
let numbers : List = ?(1, ?(2, ?(3, null)));
reverseNat(numbers); // ?(3, ?(2, ?(1, null)))
```

Parameterized:

```motoko no-repl
// Lists of naturals
type List<T> = ?(T, List<T>);

// Reverses List
func reverse<T>(l : List<T>) : List<T> {
  var current = l;
  var rev : List<T> = null;
  loop {
    switch (current) {
      case (?(head, tail)) {
        rev := ?(head, rev);
        current := tail;
      };
      case (null) {
        return rev;
      };
    };
  };
};
```

These type and function definitions generalize the previous code to work not just on lists of `Nat`s, but on lists of `T` values, for any type `T`.

You can reverse a list of numbers.

```motoko no-repl
let numbers : List<Nat> = ?(1, ?(2, ?(3, null)));
reverse<Nat>(numbers); // ?(3, ?(2, ?(1, null)))
```
But you can also reverse a list of characters:

```motoko no-repl

let chars : List<Char> = ?('a', ?('b', ?('c', null)));
reverse<Char>(numbers); // ?('c', ?('b', ?('a', null)))
```

Notice how generic types and generic functions complement each other.

## Type bounds

Generic types can use subtype constraints, ensuring that any type used in a generic function meets specific structural or concrete type requirements.

These constraints are enforced at compilation. This guarantees that the necessary properties or operations are available when the function is used, eliminating certain classes of runtime errors.

Although the concept of type bounds is often associated with [inheritance-based polymorphism](https://www.codecademy.com/learn/learn-java/modules/learn-java-inheritance-and-polymorphism/cheatsheet) in other languages, Motoko uses structural typing. This means that the subtype relationship is determined by the structure of the types rather than an explicit inheritance hierarchy. **Motoko does not support inheritance**.

This approach balances the flexibility of generic programming with the safety of compile-time checks, enabling the creation of generic functions that operate on a range of types while still enforcing specific structural or type constraints.

<!-- TODO better example that requires bounds (this one doesn't) -->
The following examples illustrate this behavior:

```motoko no-repl
func printName<T <: { name : Text }>(x : T): Text {
  debug_show(x.name);
};

let ghost = { name = "Motoko"; age = 30 };
printName(ghost);  // Allowed since 'ghost' has a 'name' field.
```

In the example above, `T <: { name : Text }` requires that any type used for `T` must be a subtype of the [record](./records.md) `{ name : Text }`, that is, it must have at least a `name` field of type [`Text`](https://mops.one/core/docs/Text). Extra fields are permitted, but the `name` field is mandatory.

Type bounds are not limited to records.
In general, the notation `T <: A` in a parameter declaration mandates that any type provided for type parameter `T` must be a subtype of the specified type `A`.
For example, it is possible to constrain a generic type to be a subtype of a primitive type.

```motoko no-repl
func max<T <: Int>(x : T, y : T) : T {
  if (x <= y) y else x
};
max<Int>(-5, -10);  // returns -5  : Int
```

Here, `T <: Int` constrains `T` to be a subtype of [`Int`](https://mops.one/core/docs/Int), ensuring that arithmetic operations are valid.

But the function can also be used to return the maximum of two `Nat`s and still produce a `Nat` (not an `Int`).

```motoko no-repl
max<Nat>(5, 10); // returns 10 : Nat
```

## Actor reference expression


The *actor reference* expression `actor <exp>` compute a reference to an actor from a text argument `<exp>`, the textual encoding of a canister id. The expression is typically combined with an (actor) type annotation,  `actor <exp> : <typ>`  that declares the expected type of the actor reference. The argument  can either be a text literal, identifier, or, when enclosed in parentheses, a more complicated expression that computes a textual id.

A simple example of using actor references is to access the management canister with textual address `"aaaaa-aa"`. Amongst other things, it has a method `raw_rand` for generating cryptographically random bytes as a `Blob`.

```motoko no-repl
persistent actor Coin {
  public func flip() : async Bool {
    let managementCanister = actor "aaaaa-aa" : actor { raw_rand : () -> async Blob };
    let entropy = await managementCanister.raw_rand();
    (entropy[0] & 1) == 1;
  }; 
}
```

A variation computes the textual canister identifier from a given principal. A call to `flipWith(p)` will succeed is called with `Principal.fromBlob("aaaaa-aa")`, but may fail with another argument, if the canister does not exist or does not have a `raw_rand` function:

```motoko no-repl
import Principal "mo:core/Principal";

persistent actor Coin {
  public func flipWith(principal : Principal) : async Bool {
    let canister = actor (Principal.toText(principal)) : actor { raw_rand : () -> async Blob };
    let entropy = await canister.raw_rand();
    (entropy[0] & 1) == 1;
  }; 
} 
```

:::caution

There is currently no way to verify the actual type of the actor is compatible with the declared type. The validity of the actor reference and any type incompatibility will be detected later, on interaction with the actor.
For this reason, you should only use actor references sparingly. It's typically safer to use explicitly imported canisters with their given actor types and avoid using `Text` or `Principal` to represent canisters in your methods (just use actor types instead).

For example, a safer variant of `flipWith` is:

```motoko no-repl
persistent actor Coin {
   public func flipWith(canister : actor { raw_rand : () -> async Blob }) : async Bool { 
       let entropy = await canister.raw_rand();
       (entropy[0] & 1) == 1;
  }; 
} 
```

This `flipWith` takes a strongly-typed actor, not a weakly-typed `Principal`.


Actor type annotations offer flexibility when working with external canisters, but there’s no guarantee that the function signatures will match at runtime. If the signatures don’t align, the calls will fail.

As another example of using actor reference expressions, we present a simple `Publisher` actor that tracks sets of subscribers by their `Principal` and uses an actor reference expression access the `notify` of each subscriber when a message is `published`:

<!-- TODO: improve or remove sample (e.g. use a set of subscribers and oneway notify that returns `()` not `async ()` -->

```motoko no-repl
import Array "mo:core/Array";
import Principal "mo:core/Principal";

actor Publisher {
    stable var subscribers : [Principal] = [];

    public shared func subscribe(subscriber : Principal) : async () {
        if (Array.find<Principal>(subscribers, func(s) { s == subscriber }) == null) {
            let newSubscribers = Array.tabulate<Principal>(
                subscribers.size() + 1,
                func(i) { if (i < subscribers.size()) subscribers[i] else subscriber }
            );
            subscribers := newSubscribers;
        };
    };

    public shared func publish(message : Text) : async () {
        for (sub in subscribers.vals()) {
            let subActor = actor(Principal.toText(sub)) : actor { notify : (Text) -> async () };
            await subActor.notify(message);
        };
    };
};
```