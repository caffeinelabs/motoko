---
title: "Subtyping"
description: "Subtyping is a fundamental concept in type systems that allows values of one type to be used wherever values of another type are expected, p"
sidebar:
  order: 14
---

Subtyping is a fundamental concept in type systems that allows values of one type to be used wherever values of another type are expected, provided
the first type is a subtype of the second.

In Motoko, the simplest subtyping relation is between `Nat` and `Int`. `Nat` is a subtype of `Int`, written `Nat <: Int`.
If you have a value of type `Nat`, you can supply it to a function taking the more general supertype `Int`.
Motoko applies subtyping when necessary to prove that a program is type correct.


:::note

Subtyping is similar to the notion of subsets, where one set `A` is a subset of another `B`, written `A ⊆ B`, if every element of the set `A` is contained in the set `B` (which may have additional elements).

Every set `A` is a subset of itself (that is `A ⊆ A`). This property of the subset relation is called [reflexivity](https://en.wikipedia.org/wiki/Reflexive_relation).

If `A` is a subset of `B` (`A ⊆ B`) and `B` is a subset of `C` (`B ⊆ C`) then `A` is a subset of `C` too (`A ⊆ C`). This property of the subset relation is called [transitivity](https://en.wikipedia.org/wiki/Transitive_relation).

Not all sets are related by subset. Take, for example, a set of natural numbers `N = {0, 1, 2, …}` and a set of integers `I = {… -1, -2, 0, 1, 2, …}`;
only one is a subset of the other (`N ⊆ I` but `I ⊈ N`, as `I` is not a subset of `N`).

:::

With types, one type `T` is a subtype of another type `U`, written `T <: U`, if every value of the type `T` is also a value of the type `U`.

Every type `T` is a trivial subtype of itself, `T <: T`. The subtype relation, like subset, is reflexive.

If `T` is subtype of `U` (`T <: U`) and `U` is a subtype of `V` (`U <: V`) then `T` is a subtype of `V` (`T <: V`) too. The subtype relation, like subset, is transitive.

The notation `T </: U` is used to indicate that `T` is not a subtype of `U`.

Subtyping has no runtime overhead. When a value is used as a supertype, no conversion takes place. The value remains the same and is simply viewed through the perspective of a different type.

In Motoko, subtyping is used to provide more flexible typing without compromising type safety.

## Variance

Motoko provides type-level syntax for constructing types from other types.

* The option type `?T` constructs an option type from `T`.
* The array type `[T]` constructs an array type from `T`.
* The mutable array type `[var T]` constructs a mutable array type from `T`.
* The object type `{… f : T; …}` constructs an object type from a field type `T`.
* The variant type `{… #f : T; …}` constructs a variant type from a field type `T`.
* The function type `T -> U` constructs a function type from the argument type `T` and result type `U`.

A type constructor is **covariant** in an argument if replacing that argument with a supertype results in a supertype of the constructed type.

It is **contravariant** if replacing the argument with a supertype results in a **subtype** of the constructed type.

It is **invariant** if replacing the argument with a supertype results in a type **unrelated** to the original constructed type.

Note that type constructors with multiple arguments can exhibit different variance behavior for each argument.

| Variance type | Description | Relationship | Motoko example |
|---------------|-------------|--------------|----------------|
| Covariant | Preserves subtyping | `F[T] <: F[U]` | `?Nat <: ?Int`. An optional  natural is also an optional integer. |
| Contravariant | Inverts subtyping | `F[U] <: F[T]` | `(Int -> ()) <: (Nat -> ())`. A function on integers is also a function on naturals |
| Invariant | No subtyping | `F[T] </: F[U]` | `[var Nat] </: [var Int]`. A mutable array of naturals is not an array of integers |

In this table:

- `T` and `U` are distinct types such that `T <: U` but not `U <: T`.

- `F[T]` represents a type constructor applied to type `T` (e.g. `?T` for option type).

## Numbers `Nat` and `Int`

[`Nat`](https://mops.one/core/docs/Nat) is a subtype of [`Int`](https://mops.one/core/docs/Int) (`Nat <: Int`), meaning a [`Nat`](https://mops.one/core/docs/Nat) value can be used where an `Int` is expected. This works because every [`Nat`](https://mops.one/core/docs/Nat) is an `Int`, but not every `Int` is a [`Nat`](https://mops.one/core/docs/Nat) (as negative numbers exist in `Int`).


```motoko no-repl
let n : Nat = 5;
let i : Int = n;        // Allowed, since `Int <: Nat`
```

```motoko no-repl
let i : Int = -5;
let n : Nat = i;        // Not allowed, since `Int </: Nat`
```

## None: the least type

In Motoko, `None` is an **empty type**. It contains no values at all. By definition, `None` is a subtype of every other type: for any type `T`, `None <: T`.

This makes `None` the least type in the subtype hierarchy.

The `None` type is used to type expressions that never produce a value, such as an infinite loop (`loop {}`).

```motoko no-repl
func impossible() : None { loop {} };

impossible() # impossible();  // Allowed, since `None <: Text`
impossible() + impossible();  // Allowed, since `None <: Nat`

if (false == true) impossible(); // Allowed, since `None <: ()`
```

Calling the function `impossible()` will force the program to enter an infinite loop and return no value.
Because `None` is the least type, it can be used wherever any other type is expected.

`None` is similar to an empty set, as it has no elements and is a subset of every other set `{} ⊆ A`.

## Any: the greatest type

In Motoko, `Any` contains all possible values. By definition, every type is a subtype of `Any`: for any type `T`, `T <: Any`.

This makes `Any` the greatest type in the subtype hierarchy.

A function that takes an argument of type `Any` can be applied to a value of any type, since all types are compatible with `Any`.

```motoko no-repl
func discard(a : Any) {};

discard(0); // Allowed, since `Nat <: Any`
discard(true); // Allowed, since `Bool <: Any`
discard("abc"); // Allowed, since `Text <: Any`
```

`Any` is similar to the universal set `U` that contains all possible elements. Every set `A` is a subset of the universal set `A ⊆ U`.

## Options

If `T <: U`, then `?T <: ?U` because option subtyping is covariant. This means an [optional value](./options.md) of a subtype can be used as an optional value of a supertype.

```motoko no-repl
let a : ?Nat = ?5;
let b : ?Int = a;     // Allowed, since `Nat <: Int` implies `?Nat <: ?Int`
```

In Motoko, the literal `null` has the type `Null`, which is a subtype of any optional type. For any type `T`, `Null <: ?T`.

This means `null` can be used as the absent value for any optional type.

```motoko no-repl
let n : Null = null;
let oi : ?Int = n;      // Allowed, since `Null <: ?Int`
let ot : ?Text = n;     // Allowed, since `Null <: ?Text`
```

## Records and objects

[Records](./records.md) and, more generally, [objects](./objects-classes.md)
support subtyping, both in the required fields and the types of those fields.

An object type `T` is a subtype of another object type `U`, if `T` requires all the fields required by `U`,
with field types that are subtypes of those in `U`.
Note that `T` may provide more fields than `U`.

```motoko no-repl
type A = { name : Text; age : Nat };
type B = { name : Text; age : Int };
type C = { name : Text };

let a : A = { name = "Ghost"; age = 25 };
let b : B = { name = "Other"; age = -1 };
let c : C = { name = "Motoko" };

let b1 : B = a; // Allowed since `age : Nat` implies `age : Int` (`Nat <: Int`).
let c1 : C = b; // Allowed, since `B` has all fields of `C`
let a1 : A = b; // Not allowed, since `age : Int` does not imply `age : Nat` (`Int </: Nat`).
let b2 : B = c; // Not allowed, because `C` is missing field `age`.
```

If the field of an object is mutable (e.g., `var age : Nat`), then any field in the supertype must also be mutable with equivalent content.

For example, consider the following object types with mutable `age` fields:

```motoko no-repl
type A = { name : Text; var age : Nat };
type B = { name : Text; var age : Int };
```

Now, neither `A <: B` nor `B <: A` holds, because the `age` fields differ in type because `Nat` and `Int` are not equivalent.

However, both `A` and `B` are still subtypes of `C`, since `C` lacks the `age` field entirely and subtyping allows dropping fields.


## Variants

[Variants](./variants.md) also support subtyping, both in the allowed fields and the types of those fields.

A variant type `T` is a subtype of another variant type `U` if every value in `T` also appears in `U`, and the associated types in `T` are subtypes of those in `U`. `T` may allow fewer fields than `U`.

In other words, `T` can define a subset of the fields defined in `U`, as long as their types are compatible (i.e., subtypes).

For example, you can define a variant `WeekDay` that is a subtype of `Day` (adding weekends):

```motoko no-repl
type WeekDay = { #mon; #tue; #wed; #thu; #fri };
type Day = { #mon; #tue; #wed; #thu; #fri; #sat; #sun};
```

Now every weekday is a day:

```motoko no-repl
let wd : WeekDay = #mon;
let d : Day = wd;  // Allowed, since `WeekDay <: Day`
```

But not the other way round:

```motoko no-repl
let d : Day = #mon;
let wd : WeekDay = d;  // Not allowed, since `Day </: WeekDay` (`d` could also be `#sat`)
```

Variants with arguments can also be related by subtyping:

```motoko no-repl
type Ok<T> = {#ok : T};
type Err<E> = {#err : E};
type Result<T, E> = {#ok : T; #err : E};

let err : Err<Text> = #err "ohoh";
let ok : Ok<Nat> = #ok 0;
let rerr : Result<Int, Text> = err;  // Allowed, since `{#err : Text} <: {#ok : Int; #err : Text}`,
                                     // that is, `Err<Text> <: Result<Int, Text>`
let rok : Result<Int, Text> = ok; // Allowed, since `{#ok : Nat} <: {#ok : Int; #err : Text}`,
                                  // that is, `Ok<Nat> <: Result<Int, Text>`
```


## Immutable arrays

[Immutable arrays (`[T]`)](./immutable-arrays.md) support covariant subtyping, meaning if `T <: U`, then `[T] <: [U]`.

If `T <: U`, then an (immutable) array of type `[T]` can be used as an array of type `[U]`.

```motoko no-repl
let nats : [Nat] = [1, 2, 3];
let ints : [Int] = nats;  // Allowed, since `Nat <: Int` we also have `[Nat] <: [Int]`.
```

## Mutable arrays

[Mutable arrays](./mutable-arrays.md) of the form `[var T]` do not support interesting subtyping.
The mutable array constructor `[var T]` is invariant in `T`.

This means that `[var T]` is a subtype of `[var U]` only if `T` and `U` are equivalent types, that is, both `T <: U` and `U <: T` must hold.

Allowing `[var T] <: [var U]` whenever `T <: U` (without requiring `U <: T`) would be unsafe, as it could allow writing values of type `U` into an array that only expects values of type `T`.

```motoko no-repl
let nats : [var Nat] = [var 1, 2, 3];
let ints : [var Int] = nats;  // Not allowed, because `[var Nat] </: [var Int]`.
```

## Functions

[Functions](./function-types.md) also support subtyping.
A function type `T1 -> T2` is a subtype of another function type `U1 -> U2` provided that:

1. The argument types are related in the opposite direction (`U1 <: T1` ).
2. The return types are related in the same direction (`T2 <: U2`).

Function subtyping is contravariant in the argument types and covariant in the return type.

A function of type `T1 → T2` can be used where a function of type `U1 → U2` is expected only if `U1 <: T1` (the expected argument is more specific) and `T2 <: U2` (the actual result is more general).

This ensures safety because the function will never be passed arguments it can’t handle, and it will always produce results that meet the expectations of the surrounding context.

As a simple example, consider the `magnitude` function that returns the absolute value, or magnitude, of an integer.

```motoko no-repl
import Int "mo:core/Int"
func magnitude(i : Int) : Nat { Int.abs(i) };
```

Its most precise type is `Int -> Nat`, but, due to subtyping, it also has types `Nat -> Nat` (making the argument more specific),
`Int -> Int`  (making the result more general) and `Int -> Int` (doing both).

```motoko no-repl
let i2n : Int -> Nat = magnitude;
let n2n : Nat -> Nat = magnitude;
let i2i : Int-> Int = magnitude;
let n2i : Nat-> Int = magnitude;
```

You might wonder what happens when one or both of the function types are generic and have type parameters.

Motoko applies a straightforward rule: two generic function types are considered subtypes only if they have the same type parameters (up to renaming), and their parameter and result types follow the usual subtyping rules.

## Modules and actors

Modules and actors in Motoko support subtyping in the same way as objects.

Specifically, a module (or actor) is a subtype of another module (or actor) with some fields removed, as long as the remaining fields have types that are related by subtyping.

In the case of actors, which can only contain shared functions as fields, this means:

* You can remove some functions.
* You can replace the types of shared functions with supertypes (according to function subtyping rules).

## Recursive and generic types

[Recursive and generic types](./advanced-types.md) can be subtypes of each other when their definitions allow it.

Consider the following recursive point types, where the second extends the first by adding a color field:

```motoko no-repl
type Point = {
  x : Int;
  move: Int -> Point
};

type ColorPoint = {
  color : {#red; #blue};
  x : Int;
  move: Int -> ColorPoint
};
```

`ColorPoint` is a subtype of `Point`:

```motoko no-repl
let cp : ColorPoint = {
  color = #red;
  x = 0;
  move = func (dx : Int) { { cp with x = cp.x + dx } }
};
let c : Point = cp;
```

This also works for recursive variants and even generic types.

For example, a tree with exclusively `#red` nodes is a subtype of a tree with both `#red` and `#black` nodes:

```motoko no-repl
type RedTree<T> = {
  #leaf;
  #red : (RedTree<T>, RedTree<T>)
};

type RedBlackTree<T> = {
  #leaf;
  #red : (RedBlackTree<T>, RedBlackTree<T>);
  #black : (RedBlackTree<T>, RedBlackTree<T>)
};

let rt : RedTree<Nat> = #leaf;
let rbt : RedBlackTree<Nat> = rt;  // Allowed, since `RedTree<Nat> <: RedBlackTree<Nat>`
```

