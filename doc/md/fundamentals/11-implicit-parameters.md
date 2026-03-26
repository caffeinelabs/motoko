# Implicit parameters

## Overview

Implicit parameters allow you to omit frequently-used function arguments at call sites when the compiler can infer them from context. This feature is particularly useful when working with ordered collections like `Map` and `Set` from the `core` library, which require comparison functions but where the comparison logic is usually obvious from the key type.
Other examples are `equal` and `toText` functions.

## Basic usage

### Declaring implicit parameters

When declaring a function, any function parameter can be declared implicit using the `implicit` type constructor:

For example, the core Map library, declares a function:

```motoko no-repl
public func add<K, V>(self: Map<K, V>, compare : (implicit : (K, K) -> Order), key : K, value : V) {
  // ...
}
```

The `implicit` marker on the type of parameter `compare` indicates the call-site can omit it the `compare` argument, provided it can be inferred the call site.

A function can declare more than on implicit parameter, even of the same name.


```motoko
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

```motoko
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let map = Map.empty<Nat, Text>();

// Without implicits - must provide compare function explicitly
Map.add(map, Nat.compare, 5, "five");

// With implicits - compare function inferred from key type
Map.add(map, 5, "five");
```
The compiler automatically finds an appropriate comparison function based on the type of the key argument.

The availabe candidates are:
* Any value named `compare` whose type matches the parameter type.

If there is no such value,
* Any field named `M.compare` declared in some module available `M`.
* If there is more than one such field, none of which is more specific than all the others, the call is ambiguous.

An ambiguous call can always be disambiguated by supplying the explicit arguments for all implicit parameters.

### Contextual dot notation

Implicit parameters dovetail nicely with the [contextual dot notation](contextual-dot).
The dot notation and implicit arguments can be used in conjunction to shorten code.

For example, since the first parameter of `Map.add` is called `self`, we can both use `map` as the receiver of `add` "method" calls
and omit the tedious `compare` argument:

```motoko
import Map "mo:core/Map";
import Nat "mo:core/Nat";

let map = Map.empty<Nat, Text>();

// Using contextual dot notation, without implicits - must provide compare function explicitly
map.add(Nat.compare, 5, "five");

// Using contextual dot nation together with implicits - compare function inferred from key type
map.add(5, "five");
```


## Working with ordered collections

The primary use case for implicit arguments is simplifying code that uses maps and sets from the `core` library.

### Map Example

```motoko
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
```motoko
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

```motoko
import Map "mo:core/Map";
import Text "mo:core/Text";

let scores = Map.empty<Text, Nat>();

// Add player scores
scores.add("Alice", 100);
scores.add("Bob", 85);
scores.add( "Charlie", 92);

// Update a score
scores.add("Bob", 95);

// Check and remove
if (scores.containsKey("Alice")) {
  scores.remove("Alice");
};

// Get size
let playerCount = scores.size()
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

1. **Direct local values** — values in the current scope whose type directly matches.
2. **Direct module fields** — fields of modules in scope (e.g., `Nat.compare`) whose type directly matches.
3. **Direct library fields** — fields of unimported libraries (requires `--implicit-package`).
4. **Derived from local values** — local functions with implicit parameters that, after stripping their own implicits and instantiating type parameters, match the required type (see [Implicit derivation](#implicit-derivation) below).
5. **Derived from module fields** — module fields with implicit parameters (e.g., `Array.compare<T>`), derived the same way.
6. **Derived from library fields** — library fields with implicit parameters (requires `--implicit-package`).
7. **Structural from local values** — local structural combiners (`__record` convention) applied to record types (see [Structural derivation](#structural-derivation) below).
8. **Structural from module fields** — module-field structural combiners.
9. **Structural from library fields** — library structural combiners (requires `--implicit-package`).

Within each tier, if multiple candidates match, the compiler picks the most specific one (by subtyping). If no unique best candidate exists, the call is rejected as ambiguous.

This ordering guarantees that direct matches are always preferred over derived ones, and local definitions take precedence over module or library definitions.

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

This works transitively: a `compare` for `[[Nat]]` is derived via `Array.compare<[Nat]>`, which needs `[Nat]` compare, which is derived via `Array.compare<Nat>`, which needs `Nat.compare` — all resolved automatically.

The resolution depth is bounded to guarantee termination. If you encounter a depth limit, you can increase it with `--implicit-derivation-depth` or provide the argument explicitly.

When derivation is attempted but fails (for example, because an inner implicit can't be resolved), the compiler includes this context in the error message, telling you which candidate was tried and which inner implicit was missing.

### Structural derivation

When an implicit is needed for a **record type**, the compiler can synthesise it automatically using a *structural combiner* — a function that converts a list of (field-name, converted-value) pairs into the target type.

A structural combiner is any function whose sole explicit parameter is named `__record` and has type `[(Text, T)] -> R` for some element type `T` and result type `R`. The `__` prefix signals to the compiler that this function acts as a record-level builder.

When the compiler is looking for an implicit of type `SomeRecord -> R` and finds a unique structural combiner for `R`, it:

1. Decomposes `SomeRecord` into its fields.
2. For each field `name : FieldType`, resolves a per-field implicit `FieldType -> T` using the same search label (e.g., `_toJson`).
3. Synthesises a wrapper function that applies each per-field implicit and assembles the resulting `[(Text, T)]` list before calling the combiner.

This makes it possible for a library to provide generic serialisation for **any** record type as long as instances exist for all field types.

#### Example: JSON serialisation

Suppose a `Json` package defines:

```motoko no-repl
public type Json = { #number : Int; #text : Text; #obj : [(Text, Json)]; /* ... */ };

// The structural combiner — named parameter triggers record-level synthesis
public func _toJson(__record : [(Text, Json)]) : Json = #obj(__record);

// Entry point using contextual dot notation
public func toJson<R>(self : R, _toJson : (implicit : R -> Json)) : Json = _toJson(self);
```

And per-type instances in companion modules:

```motoko no-repl
// IntJson.mo
public func _toJson(self : Int) : Json = #number self;
```

Any record whose fields all have `_toJson` instances can now be serialised with no boilerplate:

```motoko
import Json "mo:json/Json";
import IntJson "mo:json/IntJson";
import TextJson "mo:json/TextJson";

type Person = { name : Text; age : Int };

let p : Person = { name = "Alice"; age = 30 };
let json = p.toJson();
// Result: #obj([("name", #text "Alice"), ("age", #number 30)])
```

The compiler finds `Json._toJson(__record)` as the unique structural combiner for `Json`, then resolves `TextJson._toJson` for the `name` field and `IntJson._toJson` for the `age` field, and synthesises the wrapper automatically.

#### Naming conventions for structural combiners

The `__` prefix (double underscore) on the parameter name is the signal to the compiler. Common patterns:

- `__record : [(Text, T)] -> R` for records (supported today)
- `__tuple : [T] -> R` and `__variant : (Text, T) -> R` are reserved for future extension

The search label used to resolve per-field implicits is the same implicit parameter name used at the call site (e.g., `_toJson`). By convention, internal instances are named with a `_` prefix to distinguish them from the public entry-point function.

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

```motoko
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

```motoko
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

```motoko
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

## See also

- [Language reference](../language-manual#function-calls)
