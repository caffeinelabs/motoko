# Stable Variable Inspection with `--generate-view-queries`

The `--generate-view-queries` compiler flag instructs `moc` to auto-generate **query methods** that expose the contents of an actor's stable variables for inspection at runtime. This enables tooling, dashboards, and generic front-end UIs to browse canister state without writing boilerplate accessor code.

## Usage

```
moc --generate-view-queries ...
```

The flag is **off by default**. When enabled, the compiler examines every stable variable in the actor and, where possible, emits a corresponding query function.

## How it works

For each stable variable `id` of type `<typ>`, the compiler attempts to generate a public query method named `__id`. Generation is skipped if:

- A member named `__id` is already declared in the actor (user-defined methods take precedence).
- The name `__id` has the reserved prefix `__motoko`.

When generation proceeds, the compiler chooses one of two strategies:

### 1. `.view()` method available

If the expression `id.view()` resolves to a function with shared argument types `(T1, ..., TN)` and shared result type `R`, the compiler generates:

```motoko
public shared ({ caller }) query func __id(arg1 : T1, ..., argN : TN) : async R {
  <access-control>;
  id.view()(arg1, ..., argN)
};
```

The `.view()` call may rely on implicit arguments (such as `compare`) provided they resolve at the call site. This is the preferred strategy because it lets library authors define **logical views** of non-shared data structures. For example, a B-tree map can present its entries as paginated key-value pairs instead of exposing its internal node layout.

### 2. Shared-type fallback

If no `.view()` method is available but `<typ>` is a shared type, the compiler generates a simple accessor:

```motoko
public shared ({ caller }) query func __id() : async <typ> {
  <access-control>;
  id
};
```

### 3. No generation

### 3. Approximation to `Any`

If neither condition is met (no `.view()` and the type is not shared), the query just returns a value of the non-informative type `Any`.

## Access control

Every generated query enforces that the `caller` must be either:

1. The canister **itself** (self-calls), or
2. A **controller** of the canister.

Unauthorized callers receive a trap: `"Unauthorized caller (caller must be self or a controller)"`.

## Candid interface behaviour

- **Public interface** (custom section / `--public-metadata candid:service`): Generated view queries are **excluded**. This means upgrading a canister never requires view methods to be backward-compatible, and stable variable implementation details do not leak into the public API.
- **Local `.did` file** (`moc --idl`): Generated view queries **are included**. This allows generic front-end tools to parse the full Candid description and present a type-driven UI for data inspection.

## Writing custom `.view()` methods

A `.view()` method is a function resolved via contextual dot syntax on a stable variable's type. It must return a function whose argument and return types are all shared.

### Signature pattern

```motoko
module MyView {
  public func view<...>(self : <DataType>, ...) : (arg1 : T1, ..., argN : TN) -> R = ...
}
```

Implicit arguments (like `compare` for ordered collections) are supported and resolved automatically by the compiler.

### Example: paginated map view

```motoko
module MapView {
  public func view<K, V>(
    self : Map.Map<K, V>,
    compare : (implicit : (K, K) -> Order.Order)
  ) : (ko : ?K, count : ?Nat) -> [(K, V)] =
    func(ko, count) {
      let entries = switch ko {
        case null { self.entries() };
        case (?k) { self.entriesFrom(k) };
      };
      switch count {
        case null { entries.toArray() };
        case (?c) { entries.take(c).toArray() };
      };
    };
};
```

Given a stable variable:

```motoko
let customers : Map.Map<Text, Customer> = Map.empty();
```

The compiler generates a query with the Motoko signature:

```motoko
public shared query func __customers(ko : ?Text, count : ?Nat) : async [(Text, Customer)]
```

And corresponding Candid signature:

```candid
__customers : (ko : opt text, count : opt nat) -> (vec record { text; Customer }) query;
```

### Reusable view mixin

View modules for common core data structures (`Map`, `Set`, arrays, `List`, `Stack`, `Queue`, and their pure counterparts) can be collected into a **mixin** and included in any actor:

```motoko
import Views "views";

persistent actor {
  include Views();

  let customers : Map.Map<Text, Customer> = Map.empty();
  // __customers query is auto-generated using MapView.view
};
```

## Example: full actor

```motoko
//MOC-FLAG --generate-view-queries
import Array "mo:core/Array";

persistent actor Self {

  module ArrayView {
    public func view<V>(self : [var V]) :
      (start : Nat, count : Nat) -> [V] =
      func(start, count) {
        Array.tabulate<V>(count, func i { self[start + i] })
      };
  };

  // .view() available -> generates paginated query
  let array : [var (Nat, Text)] = [var (1, "1"), (2, "2")];

  // shared type, no .view() -> generates simple accessor
  var some_variant = #node(#leaf, 0, #leaf);
  let some_record = { a = 1; b = "hello"; c = true };

  // non-shared type, no .view() -> no query generated
  let some_mutable_record = { var a = 1 };
};
```

The actor above exposes the following generated queries:

| Stable variable        | Strategy        | Generated query                                          |
|-------------------------|-----------------|----------------------------------------------------------|
| `array`                 | `.view()`       | `__array(start : Nat, count : Nat) : async [(Nat, Text)]` |
| `some_variant`          | shared fallback | `__some_variant() : async Tree`                           |
| `some_record`           | shared fallback | `__some_record() : async {a : Nat; b : Text; c : Bool}`  |
| `some_mutable_record`   | approximated  |  `__some_mutable_record() : async Any`  |

## Limitations

- View queries are **not** part of the canister's public Candid interface and are therefore invisible to other canisters importing the actor's type.
- Only controllers and the canister itself may call the generated queries; there is currently no hook for application-level authorization.
- If a `.view()` method returns a non-shared type, the variable is treated as if no `.view()` exists (the view is silently skipped).

## Links

The sample project https://github.com/crusso/motoko-stable-viewer is a simple database application.
It provides its own, reusable `views.mo` mixin, that adds simple paginated views to `core` collections.
Its frontend uses a generic react component that renders the backend's stable variables.
The rendering is driven by the backend's Candid interface file.
