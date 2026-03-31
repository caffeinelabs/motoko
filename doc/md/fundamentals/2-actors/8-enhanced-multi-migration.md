---
sidebar_position: 8
---

# Enhanced multi-migration

Enhanced multi-migration lets you manage canister state changes over time through a series of migration modules, each stored in its own file. Instead of writing a single inline migration function, one builds up a chain of small, self-contained migrations that the compiler and runtime apply in order.

This approach is especially useful for long-lived canisters whose data shape evolves across many deployments. Each migration captures one logical change — adding a field, renaming a field, changing a type — and the compiler verifies that the entire chain is consistent.

## Overview

With enhanced multi-migration you:

1. Create a `migrations/` directory alongside your actor source.
2. Add one `.mo` file per migration, named with a timestamp prefix so they sort chronologically.
3. Each migration module exports a `public func migration({...}) : {...}` that transforms a subset of stable fields.
4. Pass `--enhanced-migration ./migrations` to `moc` when compiling.

The compiler reads all migration modules in lexicographic order, checks that they compose correctly, and compiles them into the actor. At runtime, only migrations that have not yet been applied are executed — already-applied migrations are skipped automatically.

:::note
Enhanced multi-migration requires enhanced orthogonal persistence. It cannot be combined with the inline `(with migration = ...)` syntax used for [single migration functions](./4-compatibility.md#explicit-migration-using-a-migration-function).
:::

## Getting started

### Setting up the migration directory

Create a `migrations/` directory next to your actor source. Each file in this directory is a migration module. Name files with a timestamp prefix so they sort in the intended order:

```
my-canister/
├── src/
│   └── main.mo
└── migrations/
    ├── 20250101_000000_Init.mo
    ├── 20250315_120000_AddProfile.mo
    └── 20250601_090000_RenameField.mo
```

### Writing a migration module

Each migration module must export a `public func migration` that takes a record of input fields and returns a record of output fields:

```motoko no-repl
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { name : Text; balance : Nat } {
    { name = ""; balance = 0 }
  }
}
```

The input record describes which stable fields this migration reads from the current state. The output record describes which fields this migration produces. The input field types must be compatible with the state at that point in the chain, and the output field types must ultimately be compatible with the new actor's declared stable fields. A migration only needs to mention the fields it cares about — all other stable fields are carried through unchanged.

### The actor

With enhanced multi-migration, stable actor variables are declared **without initializers**. Unlike ordinary `let` and `var` declarations in Motoko, which always require an initializing expression (e.g. `var x : Nat = 0`), an enhanced-migration actor declares only the variable's name and type:

```motoko no-repl
// src/main.mo
actor {
  var name : Text;     // no `= ...` — value comes from the migration chain
  var balance : Nat;   // likewise
  let frozen : Bool;   // `let` bindings can also be uninitialized

  public func greet() : async Text {
    "Hello, " # name # "! Your balance is " # debug_show balance
  };
}
```

The initial value of each uninitialized variable is determined entirely by the migration chain. When the canister is first deployed, every migration runs in order and the final state provides the values. On subsequent upgrades, only newly added migrations execute, but the result is the same: the migration chain — not the actor source — is the single source of truth for stable variable values.

The compiler rejects any stable variable that carries an initializer when `--enhanced-migration` is enabled. This prevents ambiguity about whether the value comes from the migration chain or from the inline expression.

:::note
Non-stable declarations (local variables inside functions, private helper fields, etc.) still require initializers as usual. Only stable actor fields use the uninitialized syntax.
:::

### Static actor body

Because the migration chain is the sole source of stable variable values, the top-level code in the actor body must be **static** — it must evaluate without immediate side effects. Arbitrary function calls, mutable updates to non-stable state, and other effectful expressions at the top level of the actor are rejected by the compiler.

The one exception is calls to functions that require `<system>` capability, such as setting up ICP timers or configuring Candid decoding limits. These calls are permitted because they do not alter stable variable state; their effects are confined to system-level configuration.

```motoko no-repl
import Timer "mo:core/Timer";

actor {
  var count : Nat;

  // Allowed: system capability call to set up a recurring timer
  ignore Timer.setTimer<system>(#seconds 5, func () : async () {
    count += 1;
  });

  // Rejected: top-level effectful expression
  // let _ = Debug.print("hello");   // ERROR — not static
};
```

This restriction ensures that the initialization of stable state is fully determined by the composition of migration functions, with no additional top-level effects in the actor body influencing the outcome.

### Compiling

Pass the migration directory to the compiler:

```bash
moc --enhanced-orthogonal-persistence \
    --default-persistent-actors \
    --enhanced-migration ./migrations \
    src/main.mo -o main.wasm
```

## Input and output fields

Each migration's `migration` function declares which fields it reads (input) and which fields it produces (output). The relationship between input and output fields determines what happens to the state:

- **Input and output** — the migration transforms this field. It reads the old value and produces a new one, potentially with a different type. The output value replaces the old one in the state.

- **Output only** — the migration introduces a new field. The field is added to the state with the value and type returned by the migration.

- **Input only** — the migration consumes and removes this field. The field is dropped from the state. Later migrations can no longer reference it.

- **Neither input nor output** — the field is untouched by this migration and carried through to the next migration (or the final actor) as-is.

For example, given the state `{a : Nat; b : Text; c : Bool}` and a migration:

```motoko no-repl
module {
  public func migration(old : { a : Nat; b : Text }) : { a : Int; d : Float } {
    { a = old.a; d = 1.0 }
  }
}
```

- `a` is in both input and output: it is transformed from `Nat` to `Int`.
- `b` is input only: it is consumed and removed from the state.
- `d` is output only: it is newly introduced.
- `c` is in neither: it is carried through unchanged.

The resulting state is `{a : Int; c : Bool; d : Float}`.

:::note
The state's field types must be compatible with the migration's input field types. The compiler checks this and rejects the program otherwise.
:::

## How migrations compose

Migrations form a chain. The compiler verifies that each migration's input is compatible with the state produced by all preceding migrations.

Consider this chain:

| Migration | Input | Output | Effect |
|-----------|-------|--------|--------|
| `Init` | `{}` | `{name : Text; balance : Nat}` | Initializes both fields |
| `AddProfile` | `{}` | `{profile : Text}` | Adds a new field |
| `RenameField` | `{name : Text}` | `{displayName : Text}` | Renames `name` to `displayName` |

After `Init`, the state is `{name : Text; balance : Nat}`.

`AddProfile` reads nothing (`{}`) and adds `profile`, so the state becomes `{name : Text; balance : Nat; profile : Text}`.

`RenameField` reads `name` from the state and produces `displayName` instead. Since `name` appears in the input but not the output, it is consumed and removed. The final state is `{displayName : Text; balance : Nat; profile : Text}`.

The actor must declare fields compatible with this final state.

:::tip
Each migration only needs to declare the fields it reads and produces. You do not need to repeat fields that pass through unchanged.
:::

## Common migration patterns

### Initializing state

The first migration in every chain initializes the actor's fields. Its input is always empty (`{}`):

```motoko no-repl
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { count : Nat; header : Text } {
    { count = 0; header = "default" }
  }
}
```

### Adding a field

To add a new field, write a migration with an empty (or minimal) input that produces the new field:

```motoko no-repl
// migrations/20250201_000000_AddEmail.mo
module {
  public func migration(_ : {}) : { email : Text } {
    { email = "" }
  }
}
```

All existing fields are carried through automatically.

### Changing a field's type

To change the type of a field, read it at its current type and produce it at the new type:

```motoko no-repl
// migrations/20250301_000000_CountToInt.mo
module {
  public func migration(old : { count : Nat }) : { count : Int } {
    { count = old.count }
  }
}
```

Here `count` changes from `Nat` to `Int`. The compiler accepts this because `Nat` is a subtype of `Int`.

### Renaming a field

To rename a field, consume the old name and produce the new name:

```motoko no-repl
// migrations/20250401_000000_RenameHeader.mo
module {
  public func migration(old : { header : Text }) : { title : Text } {
    { title = old.header }
  }
}
```

The old field `header` is removed from the state and `title` takes its place.

### Removing a field

To drop a field entirely, consume it in the input without producing it in the output:

```motoko no-repl
// migrations/20250501_000000_DropEmail.mo
module {
  public func migration(_ : { email : Text }) : {} {
    {}
  }
}
```

The corresponding actor declaration should no longer include `email`.

:::caution
Consuming a field without producing it causes data loss. The compiler issues a warning when a consumed field is not present in the final actor declaration.
:::

### Transforming data

Migrations can perform arbitrary computation. For example, splitting a full name into first and last:

```motoko no-repl
// migrations/20250601_000000_SplitName.mo
import Text "mo:base/Text";

module {
  public func migration(old : { name : Text }) : { firstName : Text; lastName : Text } {
    let parts = Text.split(old.name, #char ' ');
    let first = switch (parts.next()) { case (?f) f; case null "" };
    let last = switch (parts.next()) { case (?l) l; case null "" };
    { firstName = first; lastName = last }
  }
}
```

## Full lifecycle example

Here is how an actor's state might evolve across several deployments:

**Step 1 — Initial deployment:**

```motoko no-repl
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { a : Nat } {
    { a = 0 }
  }
}
```

```motoko no-repl
actor {
  var a : Nat;
}
```

State: `{a : Nat}`

**Step 2 — Add field `b`:**

```motoko no-repl
// migrations/20250201_000000_AddB.mo
module {
  public func migration(_ : {}) : { b : Int } {
    { b = 0 }
  }
}
```

```motoko no-repl
actor {
  var a : Nat;
  var b : Int;
}
```

State: `{a : Nat; b : Int}`

**Step 3 — Change `b` from `Int` to `Bool`:**

```motoko no-repl
// migrations/20250301_000000_ChangeBType.mo
module {
  public func migration(old : { b : Int }) : { b : Bool } {
    { b = old.b > 0 }
  }
}
```

```motoko no-repl
actor {
  var a : Nat;
  var b : Bool;
}
```

State: `{a : Nat; b : Bool}`

**Step 4 — Drop field `a`:**

```motoko no-repl
// migrations/20250401_000000_DropA.mo
module {
  public func migration(_ : { a : Nat }) : {} {
    {}
  }
}
```

```motoko no-repl
actor {
  var b : Bool;
}
```

State: `{b : Bool}`

**Step 5 — Reintroduce `a` with a new type:**

```motoko no-repl
// migrations/20250501_000000_AddAText.mo
module {
  public func migration(_ : {}) : { a : Text } {
    { a = "" }
  }
}
```

```motoko no-repl
actor {
  var a : Text;
  var b : Bool;
}
```

State: `{a : Text; b : Bool}`

Note that reintroducing `a` is allowed because it was fully dropped in step 4. The new `a : Text` is independent of the old `a : Nat`.

## Key properties

### Idempotency

Each migration is recorded after it runs. If the canister is redeployed with the same set of migrations, already-applied migrations are skipped. Redeploying is a safe no-op.

### Fast-forward upgrades

A canister does not need to be upgraded one version at a time. If a canister was last deployed at migration 3 and the new code includes migrations 1 through 10, the runtime applies migrations 4 through 10 in sequence. Skipping intermediate deployments is safe.

### Partial migrations

Each migration only mentions the fields it transforms. Unmentioned fields are carried through from the previous state unchanged. This keeps migration modules small and focused.

### Init migration required

The first migration in the chain must initialize all required fields. When a canister is deployed for the first time, all migrations run in order, starting from the first one.

## Restrictions

- Each migration file must be a module containing a `public func migration(...)`.
- The `--enhanced-migration` flag cannot be combined with the inline `(with migration = ...)` syntax.
- Enhanced multi-migration requires enhanced orthogonal persistence.
- Stable actor variables must be declared without initializers (e.g. `var x : Nat`, not `var x : Nat = 0`). The compiler rejects stable variables that carry an initializing expression.
- The actor body must be static: top-level effectful expressions and most function calls are rejected. Only calls requiring `<system>` capability (e.g. timer setup, Candid decoding configuration) are permitted.
- The state after each migration (its output merged with carried-through fields) must be compatible with the input of the next migration in the chain. The compiler rejects the program if this is not the case.
- The final state must be compatible with the actor's declared stable fields.
- Fields in the last migration's output that are not declared in the actor are rejected by the compiler.

## Usage

```bash
moc --enhanced-orthogonal-persistence \
    --default-persistent-actors \
    --enhanced-migration ./migrations \
    actor.mo -o actor.wasm
```

## See also

- [Data persistence](./3-data-persistence.md)
- [Verifying upgrade compatibility](./4-compatibility.md)
- [Enhanced orthogonal persistence](./6-orthogonal-persistence/enhanced.md)
