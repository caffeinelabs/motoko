---
title: "Type declarations"
description: "Type declarations are used for defining custom types that improve readability, reusability, and structure of the code."
sidebar:
  order: 5
---

Type declarations are used for defining custom types that improve readability, reusability, and structure of the code. They can represent records, variants, objects, or parameterized (generic) types. Motoko enforces productivity and non-expansiveness in type declarations to ensure well-formed, valid types.

## Defining a type

The `type` keyword assigns a name to a type and makes it reusable.

```motoko no-repl
// An alias for Nat
type Age = Nat;

// An alias for Text
type Username = Text;
```

These types can then be used in function definitions.

```motoko no-repl
func greet(name : Username, age : Age) : Text {
    "Hello, " # name # "! You are " # Nat.toText(age) # " years old."
}
```

They can also be used in other type definitions:

```  motoko no-repl
type Person = { username : Username; age : Age};
```

Given a similar type `User`:

```motoko no-repl
type User = {  age : Nat; username : Text };
```

Structural typing means that the types `User` and `Person` are interchangeable because their definitions are equivalent (after expanding all definitions). In other languages, `User` and `Person` might be considered incompatible types.

## Record types

In Motoko, a type can define a structured [record](../types/records.md) with labeled fields. Each field has a specific type, and you can access them using dot notation. Records are useful for organizing related data clearly and safely.

```motoko no-repl
// A reusable record
type Ghost = {
    firstName : Text;
    lastName : Text;
    age : Nat;
};

// An instance of Ghost
let motoko : Ghost = {
    firstName = "Motoko";
    lastName = "Sentinels";
    age = 30;
};
```

## Variant types

A type can also define variants, which represent different possible states or alternatives. Variants allow a value to be one of several labeled options, making it easy to handle data that can take multiple forms.

```motoko no-repl
// Allows only one of its variants at a time.
type Status = {
    #Active;
    #Inactive;

// Carries an additional Text value.
    #Banned : Text;
};

let userStatus : Status = #Active;
let bannedUser : Status = #Banned("Violation of rules");
```

## Parameterized (generic) types

Type declarations can be parameterized, allowing them to work flexibly with multiple types while ensuring type safety. This lets you create generic and reusable type definitions that adapt to different data types as needed.

```motoko no-repl
// `Box<T> is a generic type where T represents any type.
type Box<T> = {
    value: T;
};

// numberBox stores a Nat and textBox stores a Text.
let numberBox : Box<Nat> = { value = 42 };
let textBox : Box<Text> = { value = "Hello" };
```

In Motoko, all types defined within the same block can refer to each other, allowing mutually recursive type definitions.

For example, you can split the list type above into lists with even and odd numbers of elements:

```motoko no-repl
type EvenList<T> = ?(T, OddList<T>);
type OddList<T> = (T, EvenList<T>);
```

## Recursive types and productivity

Motoko allows recursive type definitions as long as they are productive. This means that any recursion in a type must pass through a constructor (such as an option (`?`), a variant, or a record field) before referring back to itself.

#### Productive recursive type example

```motoko no-repl
type List<T> = ?(T, List<T>);
```

`List<T>` defines a linked list where each node holds a value of type `T` and points to either another `List<T>` or `null` to mark the end. Since the recursion passes through a constructor (`?`), this type is productive and accepted by the compiler.

#### Non-productive recursive type example

```motoko no-repl
type C = C; // This definition immediately refers to itself
```

This type definition is considered non-productive because it is too cyclic. It never expands to a concrete type.
As a result, the compiler rejects it to prevent infinite expansion or ill-formed types.

## Expansiveness in type definitions

Motoko enforces that type definitions do not expand into ever-growing sets of types.
This property, called non-expansiveness, guarantees it is always possible to decide whether one type is a subtype of another. Without it, compilation of code may never finish.

#### Non-expansive

```motoko no-repl
// Expands without introducing an infinite set of new instances of `List<_>`.
type List<T> = ?(T, List<T>);
```

#### Expansive

```motoko no-repl
// Expands by introducing infinitely many instances of `Seq<_>`:  `Seq<[T]>`, `Seq<[[T]]>`, `Seq<[[[T]]]>`, ....
// This is expansive and not allowed.
type Seq<T> = ?(T, Seq<[T]>);
```

## Resources

- [`Record`](../types/records.md)
- [`Variant`](../types/variants.md)

<!-- TODO(future) explain variance of type definition parameters, perhaps using InvBox<T> (r/w) CoBox<T> (ro) and ContraBox<T> (wo) as examples -->

