---
title: "Implicit parameters"
description: "Using implicit parameters to pass values to functions without explicit arguments in Motoko."
sidebar:
  order: 11
---

## Overview

Implicit parameters allow you to omit frequently-used function arguments at call sites when the compiler can infer them from context. This feature is particularly useful when working with ordered collections like `Map` and `Set` from the `core` library, which require comparison functions but where the comparison logic is usually obvious from the key type.
Other examples are `equal` and `toText` functions.

## Basic usage

### Declaring implicit parameters

When declaring a function, any function parameter can be declared implicit using the `implicit` type constructor:

For example, the core `Map` library declares a function:

```motoko no-repl
public func add<K, V>(self: Map<K, V>, compare : (implicit : (K, K) -> Order), key : K, value : V) {
  // ...
}
```

The `implicit` marker on the type of parameter `compare` indicates the call-site can omit the `compare` argument, provided it can be inferred at the call site.

A function can declare more than one implicit parameter, even of the same name.


```motoko no-repl
func show<T, U>(
    self: (T, U),
    toTextT : (implicit : (toText : T -> Text)),
    toTextU : (implicit : (toText : U -> Text))) : Text {
  "(" # toTextT(self.0) # "," # toTextU(self.1) # ")"
}
```

In these cases, you can add an inner name to indicate the external names of the implicit parameters (both `toText`) and distinguish
them from the names used with the function body, `toTextT` and `toTextU`: these need to be distinct so that the body can call them.
The inner name (under `implicit`) overrides the local name of the parameter in the body.

### Calling functions with implicit arguments

When calling a function with implicit parameters, you can omit the implicit arguments if the compiler can infer them:

```motoko no-repl
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let map = Map.empty<Nat, Text>();

// Without implicits - must provide compare function explicitly
Map.add(map, Nat.compare, 5, "five");

// With implicits - compare function inferred from key type
Map.add(map, 5, "five");
```
The compiler automatically finds an appropriate comparison function based on the type of the key argument.

The available candidates are:
* Any value named `compare` whose type matches the parameter type.

If there is no such value,
* Any field named `M.compare` declared in some module available `M`.
* If there is more than one such field, none of which is more specific than all the others, the call is ambiguous.

An ambiguous call can always be disambiguated by supplying the explicit arguments for all implicit parameters.

### Contextual dot notation

Implicit parameters dovetail nicely with [contextual dot notation](contextual-dot.md).
The dot notation and implicit arguments can be used in conjunction to shorten code.

For example, since the first parameter of `Map.add` is called `self`, we can both use `map` as the receiver of `add` "method" calls
and omit the tedious `compare` argument:

```motoko no-repl
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let map = Map.empty<Nat, Text>();

// Using contextual dot notation, without implicits - must provide compare function explicitly
map.add(Nat.compare, 5, "five");

// Using contextual dot notation together with implicits - compare function inferred from key type
map.add(5, "five");
```


## Working with ordered collections

The primary use case for implicit arguments is simplifying code that uses maps and sets from the `core` library.

### Map Example

```motoko no-repl
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let inventory = Map.empty<Nat, Text>();

// Old style: explicitly pass Nat.compare
Map.add(inventory, Nat.compare, 101, "Widget");
Map.add(inventory, Nat.compare, 102, "Gadget");
Map.add(inventory, Nat.compare, 103, "Doohickey");

let item1 = Map.get(inventory, Nat.compare, 102);

// With contextual dots and implicits: compare function inferred
inventory.add(101, "Widget");
inventory.add(102, "Gadget");
inventory.add(103, "Doohickey");

let item2 = inventory.get(102);
```


### Set example

The core `Set` type also takes advantage of implicit `compare` parameters.
```motoko no-repl
import Set "mo:core/Set";
import Text "mo:core/Text";

let tags = Set.empty<Text>();

// Old style
Set.add(tags, Text.compare, "urgent");
Set.add(tags, Text.compare, "reviewed");
let hasTag1 = Set.contains(tags, Text.compare, "urgent");

// With implicits
tags.add("urgent");
tags.add("reviewed");
let hasTag2 = tags.contains("urgent");
```

### Building collections incrementally

Implicit arguments make imperative collection operations much cleaner:

```motoko no-repl
import Map "mo:core/Map";
import Text "mo:core/Text";

let scores = Map.empty<Text, Nat>();

// Add player scores
scores.add("Alice", 100);
scores.add("Bob", 85);
scores.add("Charlie", 92);

// Update a score
scores.add("Bob", 95);

// Check and remove
if (scores.containsKey("Alice")) {
  scores.remove("Alice");
};

// Get size
let playerCount = scores.size();
```

## How inference works

The compiler infers an implicit argument by:

1. Examining the types of the explicit arguments provided.
2. Looking for all candidate values for the implicit argument in the current scope that match the required type and name.
3. From these, selecting the best unique candidate based on type specificity.

If there is no unique best candidate the compiler rejects the call as ambiguous.

If a callee takes several implicit parameters, either all implicit arguments must be omitted, or all explicit and implicit arguments must be provided at the call site,
in their declared order.

### Resolution order

The compiler searches for implicit arguments in the following order, stopping at the first tier that produces a unique match:

1. **Direct**: values whose type directly matches:
   1. Local values in the current scope.
   2. Module fields of modules in scope (e.g., `Nat.compare`).
   3. Fields of unimported modules (requires `--implicit-package`).
2. **Derived**: functions with implicit parameters that, after stripping their own implicits and instantiating type parameters, match the required type (see [Implicit derivation](#implicit-derivation) below):
   1. Local values in the current scope.
   2. Module fields (e.g., `Array.compare<T>`).
   3. Fields of unimported modules (requires `--implicit-package`).
3. **Structural** — structural combiners (`__record`, `__tuple` convention) applied to record or tuple types (see [Structural derivation](#structural-derivation) below):
   1. Local values in the current scope.
   2. Module fields.
   3. Fields of unimported modules (requires `--implicit-package`).

Within each tier, if multiple candidates match, the compiler picks the most specific one (by subtyping). If no unique best candidate exists, the call is rejected as ambiguous.

This ordering guarantees that direct matches are always preferred over derived ones, and local definitions take precedence over imported or unimported module definitions.

### Implicit derivation

When no direct match exists, the compiler can **derive** an implicit argument from a function that itself has implicit parameters. This eliminates the need for boilerplate wrapper functions. The candidate function can be polymorphic (the compiler infers the type instantiation) or monomorphic.

For example, suppose `Array.compare` is declared as:

```motoko no-repl
public func compare<T>(a : [T], b : [T], compare : (implicit : (T, T) -> Order)) : Order
```

and a function requires an implicit `compare : ([Nat], [Nat]) -> Order`. Without derivation, you would need to write a wrapper:

```motoko no-repl
module MyArray {
  public func compare(a : [Nat], b : [Nat]) : Order {
    Array.compare(a, b) // resolves inner `compare` to Nat.compare
  };
};
```

With derivation, the compiler handles this automatically. It recognizes that `Array.compare<Nat>`, after removing its implicit `compare` parameter and instantiating `T := Nat`, has the right type. It then recursively resolves the inner implicit (`Nat.compare`) and synthesizes the wrapper for you.

This works transitively: a `compare` for `[[Nat]]` is derived via `Array.compare<[Nat]>`, which needs `[Nat]` compare, which is derived via `Array.compare<Nat>`, which needs `Nat.compare`, all resolved automatically.

The resolution depth is bounded to guarantee termination. If you encounter a depth limit, you can increase it with `--implicit-derivation-depth` or provide the argument explicitly.

When derivation is attempted but fails (for example, because an inner implicit can't be resolved), the compiler reports which inner implicits were missing and, when applicable, a hint about which module to import.

### Structural derivation

When an implicit is needed for a **record or tuple type**, the compiler can synthesize it automatically using a *structural combiner* — a function whose single parameter name begins with `__` and encodes the structural decomposition kind. Structural combiners must not have implicit parameters.

Two structural kinds are supported, distinguished by the combiner's parameter name:

| Parameter name | Combiner type              | Implicit argument type                   | Description                                    |
|----------------|----------------------------|------------------------------------------|------------------------------------------------|
| `__record`     | `[(Text, () -> E)] -> R`   | `Rec -> R` or `(Rec, Rec) -> R`          | Record: one or two records, arity from implicit|
| `__tuple`      | `[() -> E] -> R`           | `(A, B, ...) -> R` or `((A,B,...), (A,B,...)) -> R` (≥ 2 elements) | Tuple: one implicit per element |
| `__variant`    | —                          | —                                        | Reserved for future extension                  |

Each per-field/element result is wrapped in a **thunk** (`() -> E`), giving the combiner full control over evaluation order. Combiners that need all values (like serialization) simply call every thunk. Combiners that can short-circuit (like comparison) can stop early — remaining thunks are never evaluated.

The search label used to resolve per-element implicits is the same as the implicit parameter name at the call site.

:::caution
Motoko has no type abstraction (no newtypes or private types), so a named type that expands to a record — including stdlib containers like `Map`, `Set`, or `Buffer` — is structurally indistinguishable from a plain data record and may be decomposed into its internal fields by structural derivation; provide a dedicated instance (e.g. `MapJson`) to take precedence over structural synthesis for such types.
:::

#### Unary record derivation (`__record`)

When the compiler is looking for an implicit of type `SomeRecord -> R` and finds a unique structural combiner for `R` (parameter named `__record`, type `[(Text, () -> E)] -> R`), it:

1. Decomposes `SomeRecord` into its fields (in lexicographic order).
2. For each field `name : FieldType`, resolves a per-field implicit of type `FieldType -> E` using the same search label.
3. Synthesises a wrapper: `func($r) { combiner([("f1", func() { inst1($r.f1) }), ...]) }`.

This makes it possible for a library to provide generic serialization for **any** record type as long as instances exist for all field types.

##### Example: JSON serialization

Suppose a `Json` package defines a type, a structural combiner, and an entry point:

```motoko no-repl
public type Json = { #number : Int; #text : Text; #obj : [(Text, Json)]; /* ... */ };

// Structural combiner — __record parameter name triggers record-level synthesis.
// Each field is a thunk; serialization evaluates all of them.
public func encode(__record : [(Text, () -> Json)]) : Json =
  #obj(__record.map(func((k, v)) = (k, v())));

// Entry point using contextual dot notation
public func toJson<R>(self : R, encode : (implicit : R -> Json)) : Json = encode(self);
```

And per-type instances in companion modules:

```motoko no-repl
// IntJson.mo
public func encode(self : Int) : Json = #number self;
```

Any record whose fields all have an `encode` instance can now be serialised with no boilerplate:

```motoko
import Json "mo:json/Json";
import IntJson "mo:json/IntJson";
import TextJson "mo:json/TextJson";

type Person = { name : Text; age : Int };

let p : Person = { name = "Alice"; age = 30 };
let json = p.toJson();
// Result: #obj([("name", #text "Alice"), ("age", #number 30)])
```

The compiler finds `Json.encode(__record)` as the unique structural combiner for `Json`, resolves per-field `encode` instances from `TextJson` and `IntJson`, and synthesizes the wrapper automatically.

#### Binary record derivation

When the compiler is looking for an implicit of type `(Rec, Rec) -> R` where `Rec` is a record type and both arguments have the same type, it searches for a `__record` combiner for `R` — the same combiner that handles the unary case. The arity is determined entirely by the implicit argument's type; the combiner itself is unaware of it.

The compiler synthesizes a binary wrapper:

```
func($r1, $r2) { combiner([("f1", func() { inst1($r1.f1, $r2.f1) }), ...]) }
```

Each per-field implicit has type `(FieldType, FieldType) -> E`, resolved recursively with the same search label. This allows binary operations like comparison or equality to be derived field-by-field from a single `__record` combiner.

##### Example: lexicographic comparison

```motoko
import Array "mo:core/Array";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import Order "mo:core/Order";

// __record combiner: fold field-wise Order values, short-circuiting at first non-equal.
// Thunks enable genuine short-circuiting — remaining fields are never evaluated.
func compare(__record : [(Text, () -> Order.Order)]) : Order.Order {
  for ((_, ordThunk) in __record.vals()) {
    let ord = ordThunk();
    if (ord != #equal) return ord
  };
  #equal
};

type Person = { name : Text; age : Nat };

// Array.sort uses (implicit : (T, T) -> Order.Order) — derived from __record (binary path).
// Fields resolved: age → Nat.compare, name → Text.compare (lexicographic order).
let people : [Person] = [{ name = "Carol"; age = 30 }, { name = "Bob"; age = 25 }];
let sorted = people.sort();
// sorted[0] = { name = "Bob"; age = 25 }  (age 25 < 30)
```

Nested record types are handled automatically: a `Team` with a `Person` field will derive `compare` for `Team` by first deriving `compare` for `Person` at depth+1.

#### Tuple derivation (`__tuple`)

When the compiler is looking for an implicit of type `(A, B, ...) -> R` (a tuple domain with at least two elements), it searches for a structural combiner whose parameter is named `__tuple` and has type `[() -> E] -> R`.

When found, the compiler synthesizes a wrapper:

```
func($t) { combiner([func() { inst0($t.0) }, func() { inst1($t.1) }, ...]) }
```

Each per-element implicit has type `ElemType_i -> E`, resolved positionally using the same search label.

#### Binary tuple derivation

Like `__record`, the `__tuple` combiner also supports binary implicit arguments. When the implicit argument has type `((A, B, ...), (A, B, ...)) -> R` (two arguments of the same tuple type with ≥ 2 elements), the compiler synthesizes a binary wrapper:

```
func($t1, $t2) { combiner([func() { inst0($t1.0, $t2.0) }, func() { inst1($t1.1, $t2.1) }, ...]) }
```

Each per-element implicit has type `(ElemType_i, ElemType_i) -> E`. This enables element-wise binary operations like comparison or equality over tuples.

##### Example: tuple description

```motoko
// __tuple combiner: join per-element descriptions (evaluates all thunks)
func describe(__tuple : [() -> Text]) : Text {
  var s = "("; var first = true;
  for (t in __tuple.vals()) {
    if (not first) { s #= ", " };
    s #= t(); first := false
  };
  s #= ")"; s
};

module TextDesc { public func describe(self : Text) : Text = self };
module NatDesc  { public func describe(self : Nat)  : Text = debug_show self };

func inspect<T>(x : T, describe : (implicit : T -> Text)) : Text = describe(x);

assert inspect(("hello", 42 : Nat)) == "(hello, 42)";
```

#### Disambiguation: binary vs unary when both `__record` and `__tuple` are in scope

Having `__record` and `__tuple` combiners in scope simultaneously is safe — the compiler picks the right path by inspecting the **number of arguments** in the implicit argument's function type. The dispatch depends on where the tuple appears in the source, not on what the type expands to:

- `implicit : (X, X) -> T` — the inline tuple `(X, X)` is flattened into two separate args. The compiler sees a **two-argument** function, checks that both args are the same type, and uses the binary path: `__record` if `X` is a record type, `__tuple` if `X` is a tuple type (≥ 2 elements).
- `implicit : P -> T` where `P` is a **type alias** for `(A, B, ...)` — `P` is not a tuple in the source, so it stays as a single arg. The compiler sees a **one-argument** function, promotes `P` to a tuple, and uses the `__tuple` combiner (unary path).

In practice: write `(X, X) -> T` directly as two args to trigger the binary path. Going through a type alias `type Pair = (R, R)` and writing `Pair -> T` will route to `__tuple` (unary) instead.

### Supported types

The core library provides comparison functions for common types:

- `Nat.compare` for `Nat`
- `Int.compare` for `Int`
- `Text.compare` for `Text`
- `Char.compare` for `Char`
- `Bool.compare` for `Bool`
- `Principal.compare` for `Principal`
- etc.

Other implicit parameters declared by the core library are `equals : (implicit : (T, T) -> Bool)` and `toText: (implicit : T -> Text)`.

## Explicitly providing implicit arguments

You can always provide implicit arguments explicitly when needed:

```motoko no-repl
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import {type Order} "mo:core/Order";

// Custom comparison function for reverse ordering
func reverseCompare(a : Nat, b : Nat) : Order {
  Nat.compare(b, a)
};

let reversedMap = Map.empty<Nat, Text>();
// Explicitly provide the comparison function
reversedMap.add(reverseCompare, 5, "five");
reversedMap.add(reverseCompare, 3, "three");
```

This is useful when:
- Using custom comparison logic
- Working with custom types that have multiple possible orderings
- Improving code clarity in complex scenarios

## Custom types

To use implicit arguments with your own custom types, define a comparison function:

```motoko no-repl
import Map "mo:core/Map";
import Text "mo:core/Text";
import {type Order} "mo:core/Order";

type Person = {
  name : Text;
  age : Nat;
};

module Person {
  public func compare(a : Person, b : Person) : Order {
    Text.compare(a.name, b.name)
  };
};

// Now works with implicits
let directory = Map.empty<Person, Text>();
directory.add({ name = "Alice"; age = 30 }, "alice@example.com");
directory.add({ name = "Bob"; age = 25 }, "bob@example.com");

let email = directory.get({ name = "Alice"; age = 30 });
```

## Best practices

1. **Use implicits for standard types**: When working with `Nat`, `Text`, `Int`, `Principal`, and other primitive types, let the compiler infer the comparison function.

2. **Be explicit with custom logic**: When using non-standard comparison logic, explicitly provide the comparison function for clarity.

3. **Name comparison functions consistently**: Follow the convention of `ModuleName.compare` to ensure proper inference.

4. **Consider readability**: While implicits reduce boilerplate, explicit arguments may be clearer in some contexts, especially when teaching or documenting code.

5. **Collections benefit most**: The repeated operations on `Map` and `Set` from `core` particularly benefit from implicit arguments since you call these functions frequently.

6. Don't go wild with implicit parameters. Use them sparingly.

## Migration from explicit arguments

Existing code with explicit comparison functions will continue to work. You can adopt implicit arguments gradually:

```motoko no-repl
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let data = Map.empty<Nat, Text>();

// Both styles work simultaneously
Map.add(data, Nat.compare, 1, "one");  // Explicit
Map.add(data, 2, "two");                // Implicit
Map.add(data, 3, "three");              // Implicit
```

There is no need to update existing code unless you want to take advantage of the cleaner syntax.

## Performance considerations

Implicit arguments are resolved at compile time.
- For direct matches, the resulting code is identical to explicitly passing the argument.
- For derived implicits, the compiler synthesizes a wrapper function at each call site. This creates a small overhead per call site, which could be mitigated by caching in the future. For now, if this becomes a performance issue, consider defining the function explicitly so all call sites share a single definition.
- For `__record` structural derivation, the synthesized wrapper invokes one implicit per record field (two invocations per field for the binary path), so runtime cost scales linearly with record width. For `__tuple`, cost scales with tuple arity. For hot paths with wide types, consider writing the combiner explicitly.

## See also

- [Language reference](../language-manual#function-calls)
