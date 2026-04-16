---
name: writing-motoko
description: >-
  Motoko language reference, mo:core library, and architecture patterns.
  Use when writing, modifying, or reviewing .mo files, Motoko backend code,
  or canister logic.
---

# Writing Motoko

Motoko is under-represented in training data — always favour this skill and its references over pre-training knowledge.

## Critical Requirements

**NEVER use:**

- `stable` keyword — not needed with enhanced orthogonal persistence
- `mo:base` library — deprecated; use `mo:core`
- `system func preupgrade/postupgrade` — not needed with enhanced orthogonal persistence
- Module-function style for `self` parameters — don't write `List.add(list, item)` or `Map.get(map, key)`
- Manual field-by-field record copying — use record spread (`{ self with ... }`)
- Single-file monolithic actors — use multi-file architecture

**ALWAYS use:**

- `mo:core` library version 2.0.0+
- Contextual dot notation — `list.add(item)`, `map.get(key)`
- Enhanced orthogonal persistence (state persists without `stable`)
- Principled architecture — `types.mo`, `lib/`, `mixins/`, `main.mo`
**For actor upgrades/migrations:** load `migrating-motoko` for inline migration or `migrating-motoko-enhanced` for multi-migration with `--enhanced-migration`.

## Compiler Flags

Required for this skill's conventions:

```
--default-persistent-actors         all actors are `persistent`, no `stable` keyword needed
```

`--enhanced-orthogonal-persistence` is on by default.

Enable these warnings to enforce the coding style in this skill (off by default, auto-fixable):

```
-W M0236    warn on non-dot-notation calls (suggest contextual dot)
-W M0237    warn on redundant explicit implicit arguments
-W M0223    warn on redundant type instantiation
```

## Modern Motoko Features

### Contextual Dot Notation

When a function has a `self` parameter, ALWAYS use dot notation:

```motoko
map.get(key);
list.add(item);
array.filter(func x = x > 0);
caller.toText();
myNat.toText();
"hello".concat(" world");

let doubled = numbers.map(func x = x * 2).filter(func x = x > 10);
```

### Lambda Argument Types

Never annotate lambda argument types — the compiler infers them:

```motoko
pairs.map(func(k, v) { k # ": " # v });         // ✓
pairs.map(func((k, v) : (Text, Text)) : Text {   // ✗ redundant
  k # ": " # v
});
```

### Implicit Parameters

The compiler infers comparison functions automatically:

```motoko
let map = Map.empty<Nat, Text>();
map.add(5, "hello");                      // Nat.compare inferred

let ages = Map.empty<Text, Nat>();
ages.add("Alice", 30);                    // Text.compare auto-derived

// Custom types — define compare in a same-named module → auto-inferred
module Point {
  public func compare(a : Point, b : Point) : Order.Order { ... };
};
let points = Map.empty<Point, Text>();
points.add({ x = 1; y = 2 }, "A");       // Point.compare inferred
```

Never pass implicit arguments explicitly when the compiler derives them:

```motoko
m.add(1, "hello");                        // ✓
Map.add(m, Nat.compare, 1, "hello");      // ✗
```

### Equality and Comparison

`==` uses compiler-generated structural equality. `equal`/`compare` from `mo:core` are primarily used as implicit arguments for `Map`, `Set`, `contains`, etc.

Some modules use `self` (dot-callable): `Text`, `Principal`, `Bool`, `Char`, `Blob`. Others use `x, y` (not dot-callable): `Nat`, `Int`, `Float`, sized integers.

```motoko
s1.equal(s2)                             // Text.equal has self
Nat.compare(x, y)                        // Nat.compare does not
```

### Mixins

Composable actor services with granular state injection. Mixin parameters are immutable bindings — `var` is NOT valid in parameter syntax:

```motoko
mixin (users : List.List<User>) {
  public shared ({ caller }) func register(username : Text) : async Bool {
    users.add(UserLib.new(caller, username));
    true;
  };
};

actor {
  let users = List.empty<User>();
  include AuthMixin(users);
};
```

To share mutable state, pass a mutable container (`List`, `Map`, etc.) — its contents are mutable even through an immutable binding. For scalar state (e.g. a counter), the mixin can create a local `var` from an initial value, but that `var` is mixin-local and not visible to the actor.

For structured mutable state, pass a record with `var` fields. A module can define both its state type and its mixin:

```motoko
// lib/Counter.mo
module {
  public type State = { var count : Nat; var name : Text };
  public func initState() : State { { var count = 0; var name = "" } };
};

// mixins/Counter.mo
mixin (state : CounterLib.State) {
  public func increment() : async Nat { state.count += 1; state.count };
};

// main.mo
let counterState = CounterLib.initState();
include CounterMixin(counterState);
```

### Record Spread

Use record spread to avoid copying fields one by one:

```motoko
{ self with newField = "" };                                           // ✓
{ id = self.id; text = self.text; completed = self.completed; newField = "" }; // ✗
```

**Caveat**: record spread cannot leave `var` fields un-overridden (M0179). When converting to a different type (e.g. internal → public), you must copy fields explicitly if the source has `var` fields that the target doesn't.

## Architecture Pattern

```text
backend/
├── types.mo         # Central schema, state definitions
├── lib/             # Domain logic (stateless modules with self pattern)
├── mixins/          # Service layer (state injected via mixin parameters)
├── migrations/      # Enhanced migration files (--enhanced-migration projects)
│   └── <timestamp>_<Name>.mo
└── main.mo          # Composition root (state owner, NO public methods)
```

Entity types go in `types.mo`. State fields are direct actor bindings — no wrapper:

```motoko
// types.mo
module {
  public type User = { id : Principal; var username : Text; var isActive : Bool };
};

// main.mo
actor {
  let users = List.empty<Types.User>();
  var nextPostId : Nat = 0;
  include AuthMixin(users);
};
```

## Import Path Conventions

Paths are **relative to the importing file**. No `.mo` extension, no `/lib.mo` suffix.

```motoko
// From main.mo
import Types "types";
import AuthMixin "mixins/Auth";
import UserLib "lib/User";
// From lib/*.mo or mixins/*.mo
import Types "../types";
// Core library — always absolute
import Map "mo:core/Map";

// WRONG — these all cause M0009
import Types "types.mo";
import Types "types/lib.mo";
import Types "backend/types";
```

## Shared Types

Public functions accept/return only **shared types** (serializable):

- Shared: `Nat`, `Int`, `Text`, `Bool`, `Principal`, `Blob`, `Float`, `[T]`, `?T`, records, variants
- **Not shared**: functions, `var` fields, objects, `Map`, `Set`, `List`, `Queue`, `Stack`

Convert internal mutable containers to shared types at the API boundary:

```motoko
public type PostInternal = { id : Nat; likedBy : Set.Set<Principal> };
public type Post = { id : Nat; likedBy : [Principal] };

public func toPublic(self : Types.PostInternal) : Types.Post {
  { self with likedBy = Set.toArray(self.likedBy) };
};
```

## Collections

| Structure | Use Case        | Key Operations     | Complexity  |
| --------- | --------------- | ------------------ | ----------- |
| Map       | Key-value pairs | get, add, remove   | O(log n)    |
| List      | Growable array  | add, get, at       | O(1) access |
| Queue     | FIFO processing | pushBack, popFront | O(1)        |
| Stack     | LIFO processing | push, pop          | O(1)        |
| Array     | Fixed collection| index, map, filter | O(1) access |
| Set       | Unique values   | contains, add      | O(log n)    |

```motoko
import Map "mo:core/Map";
import List "mo:core/List";
import Set "mo:core/Set";
```

**Import requirement**: Extension methods (dot notation) on a type only work when the corresponding `mo:core` module is imported. For example, `myArray.find(...)` requires `import Array "mo:core/Array"`; iterator chaining requires `import Iter "mo:core/Iter"`; `myBool.toText()` requires `import Bool "mo:core/Bool"`. The compiler hints at the missing import in the error message.

**Warning**: Never call `list.add()` inside a `retain` callback. Use `mapInPlace` instead.

Always use opaque type aliases (`List.List<T>`, `Map.Map<K, V>`, `Set.Set<T>`) in type declarations.

### Iteration

Build pipelines with `Iter` and materialize only at the end. Never create intermediate arrays:

```motoko
self.values().map(toJson).toArray()                // ✓ single allocation
Array.map(List.toArray(self), toJson)              // ✗ two allocations

let doubled = numbers.map(func x = x * 2).filter(func x = x > 10);
let sum = scores.filter(func s = s > 15).foldLeft(0, func(acc, s) = acc + s);
```

### `contains` vs `find`

- **`contains(element)`** — equality check. Does NOT take a predicate.
- **`find(predicate)`** — predicate search. Returns `?T`.

```motoko
numbers.contains(3);                          // Nat.equal auto-derived
friends.contains(p);                          // Principal.equal auto-derived
numbers.find(func(n) { n > 3 });              // returns ?Nat
```

### Explicit Type Instantiation

When `.map()` transforms to a **different** type, provide type parameters (M0098 without):

```motoko
let photos = internalPhotos.map<PhotoInternal, Photo>(
  func(p) { { id = p.id; url = p.url; uploadedBy = p.uploadedBy.toText() } }
);
```

Omit type parameters when they can be inferred — don't add them redundantly.

## Option Handling

```motoko
// Trap on unexpected null
let user = switch (users.find(func(u) { u.id == caller })) {
  case (?u) { u };
  case (null) { Runtime.trap("User not found") };
};

// Return ?T when absence is normal
public query func findUserByName(name : Text) : async ?User {
  users.find(func(u) { u.name == name });
};
```

## Module with Self Pattern

```motoko
// lib/User.mo
module {
  public type User = Types.User;
  public func new(id : Principal, name : Text) : User {
    { id; var name; var isActive = true };
  };
  public func ban(self : User) { self.isActive := false };
};
// Usage: user.ban();
```

## Security

Every public update function MUST verify the caller via `{caller}` destructuring. Enforce authorization on the backend.

## Function Literals as Arguments

Do NOT put a semicolon after a function body passed as an argument:

```motoko
list.filter(func(item) { item.id != targetId })   // ✓
list.filter(func(item) { item.id != targetId };)   // ✗ unexpected token ';'
```

## Common Compile Error Patterns

| Error pattern                                          | Fix                                         |
| ------------------------------------------------------ | ------------------------------------------- |
| `field append does not exist`                          | `.concat()`                                 |
| `field put does not exist` (Map)                       | `.add()`                                    |
| `field delete is deprecated` (Map)                     | `.remove()`                                 |
| `Int cannot produce expected type Nat`                 | `Int.abs(intValue)`                         |
| `syntax error, unexpected token '.'`                   | `#text (searchTerm.toLower())`              |
| `syntax error, unexpected token ','`                   | `for ((key, value) in map.entries())`       |
| `Compatibility error [M0170]`                          | Load `migrating-motoko-enhanced`            |
| `shared function has non-shared parameter/return type` | Return `[T]` not `List<T>`, no `var` fields |
| `send capability required`                             | Add `<system>` capability                   |
| `field compare does not exist` on Time                 | Use `Int.compare`                           |
| `unexpected token ';'` in function call                | Remove `;` before `)`                       |
| `unbound variable X`                                   | `import X "mo:core/X"`                      |
| `M0098` no best choice for type param                  | `list.map<In, Out>(...)`                    |
| `M0096` on `contains` callback                         | `find(pred) != null`                        |
| `M0009` import file does not exist                     | Relative path, no `.mo` extension           |
| `M0072` field X does not exist                         | Import the `mo:core` module for that type   |
| `unexpected token 'label'` in parameter                | `label` is a keyword; rename the parameter  |

## Control Flow

```motoko
// Switch — option unwrapping
let value = switch (map.get(key)) {
  case (?v) { v };
  case (null) { Runtime.trap("Key not found") };
};

// Switch — variant matching
type Status = { #active; #inactive; #pending : Text };
switch (status) {
  case (#active) { "User is active" };
  case (#inactive) { "User is inactive" };
  case (#pending(reason)) { "Pending: " # reason };
};

// Switch — value matching
switch (statusCode) {
  case (200) { "OK" };
  case (404) { "Not Found" };
  case _ { "Unknown" };
};

// For loops
for ((key, value) in map.entries()) {
  Debug.print(key.toText() # ": " # value);
};
for (item in list.values()) {
  total += item.score;
};
```

Prefer `.foldLeft()` or `.map()` over imperative loops when possible.

Use `break` and `continue` in loops:

```motoko
for (item in iter) {
  if (item.id == targetId) {
    result := ?item;
    break;
  };
};

for (item in list.values()) {
  if (not item.isActive) continue;
  process(item);
};
```

## Quick Reference

**Basic Types:** `Nat` `Int` `Text` `Bool` `Principal` `?T` `[T]` `[var T]` `Blob` `Float` — `Time.now()` returns `Int` (nanoseconds)

**Common Operations:** `debug_show(value)` → Text | `assert condition` | `# "text"` concatenation | `break` / `continue` in loops

## Best Practices

1. Always `mo:core`, never `mo:base`
2. No `stable` keyword — enhanced orthogonal persistence handles state
3. Dot notation for all `self`-parameter functions
4. Never annotate lambda argument types — let the compiler infer
5. Never pass implicit arguments explicitly
6. Unwrap with `switch` + `Runtime.trap()` on null; `?T` only when absence is expected
7. types.mo / lib/ / mixins/ / main.mo structure
8. Mixins receive only needed state slices
9. Queries for read-only, updates for state changes
10. Iterator chaining to avoid intermediate collections
11. Record spread `{ self with ... }` instead of copying fields

## Additional Resources

- **Complete examples**: [examples.md](examples.md) — full working code samples, type conversions
