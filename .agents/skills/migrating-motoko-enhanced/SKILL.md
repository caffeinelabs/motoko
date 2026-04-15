---
name: migrating-motoko-enhanced
description: >-
  Enhanced multi-migration for Motoko actors. Use when writing migration files,
  upgrading canister state, changing actor field types, or working with the
  migrations/ directory and --enhanced-migration flag.
---

# Enhanced Multi-Migration

Manage canister state evolution through a chain of migration modules. Each migration captures one logical change (add, rename, drop, transform a field) and the compiler verifies the entire chain is consistent.

## When to Use

- Adding, removing, or renaming persistent actor fields
- Changing a field's type
- Restructuring state across canister upgrades
- Project has `--enhanced-migration=migrations` in canister args in `mops.toml`

## Critical Rules

- **Never use** `stable` keyword, `preupgrade`/`postupgrade`, or inline `(with migration = ...)`
- Actor variables are declared **without initializers** — values come from the migration chain
- The actor body must be **static** (no top-level side effects except `<system>` calls like timers)
- Each migration file exports `public func migration({...}) : {...}`
- Files are applied in **lexicographic order** — use timestamp prefixes

## Directory Layout

```text
backend/
├── main.mo
├── types.mo
├── lib/
├── mixins/
└── migrations/
    ├── 20250101_000000_Init.mo
    ├── 20250315_120000_AddProfile.mo
    └── 20250601_090000_RenameField.mo
```

## Actor Syntax

With enhanced migration, actor variables have no initializer:

```motoko
actor {
  var name : Text;       // value comes from migration chain
  var balance : Nat;     // likewise
  let frozen : Bool;     // let bindings can also be uninitialized

  public func greet() : async Text {
    "Hello, " # name # "! Balance: " # debug_show balance;
  };
};
```

## Migration Module Structure

Each migration module takes a record of input fields and returns a record of output fields:

```motoko
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { name : Text; balance : Nat } {
    { name = ""; balance = 0 }
  }
}
```

## Input / Output Field Semantics

| Field appears in | Effect |
| ---------------- | ------ |
| Input and output | Field is transformed (old value read, new value produced) |
| Output only      | New field added to state |
| Input only       | Field consumed and removed from state |
| Neither          | Field carried through unchanged |

Given state `{a : Nat; b : Text; c : Bool}` and migration:

```motoko
module {
  public func migration(old : { a : Nat; b : Text }) : { a : Int; d : Float } {
    { a = old.a; d = 1.0 }
  }
}
```

- `a`: transformed `Nat → Int`
- `b`: consumed (removed)
- `c`: carried through unchanged
- `d`: newly introduced
- Result: `{a : Int; c : Bool; d : Float}`

## Common Patterns

### Initialize state (first migration, always required)

```motoko
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { count : Nat; header : Text } {
    { count = 0; header = "default" }
  }
}
```

### Add a field

```motoko
// migrations/20250201_000000_AddEmail.mo
module {
  public func migration(_ : {}) : { email : Text } {
    { email = "" }
  }
}
```

### Add an optional field

```motoko
module {
  public func migration(_ : {}) : { assignee : ?Principal } {
    { assignee = null }
  }
}
```

### Change a field's type

```motoko
// migrations/20250301_000000_CountToInt.mo
module {
  public func migration(old : { count : Nat }) : { count : Int } {
    { count = old.count }
  }
}
```

### Rename a field

```motoko
// migrations/20250401_000000_RenameHeader.mo
module {
  public func migration(old : { header : Text }) : { title : Text } {
    { title = old.header }
  }
}
```

### Remove a field

```motoko
// migrations/20250501_000000_DropEmail.mo
module {
  public func migration(_ : { email : Text }) : {} {
    {}
  }
}
```

### Transform data (split a field)

```motoko
// migrations/20250601_000000_SplitName.mo
import Text "mo:core/Text";

module {
  public func migration(old : { name : Text }) : { firstName : Text; lastName : Text } {
    let parts = old.name.split(#char ' ');
    let first = switch (parts.next()) { case (?f) f; case (null) "" };
    let last = switch (parts.next()) { case (?l) l; case (null) "" };
    { firstName = first; lastName = last }
  }
}
```

### Bool to variant

```motoko
module {
  public func migration(old : { var completed : Bool }) : { var status : { #pending; #completed } } {
    { var status = if (old.completed) { #completed } else { #pending } }
  }
}
```

### Map over a collection

```motoko
import Map "mo:core/Map";

module {
  type OldTask = { id : Nat; title : Text; var completed : Bool };
  type NewTask = { id : Nat; title : Text; var status : { #pending; #completed } };

  public func migration(old : { var tasks : Map.Map<Nat, OldTask> })
    : { var tasks : Map.Map<Nat, NewTask> } {
    let tasks = old.tasks.map<Nat, OldTask, NewTask>(
      func(_, task) {
        {
          id = task.id;
          title = task.title;
          var status = if (task.completed) { #completed } else { #pending };
        }
      }
    );
    { var tasks }
  }
}
```

### Add field to each record in a Map

```motoko
import Map "mo:core/Map";

module {
  type OldUser = { name : Text; email : Text };
  type NewUser = { name : Text; email : Text; bio : Text };

  public func migration(old : { users : Map.Map<Nat, OldUser> })
    : { users : Map.Map<Nat, NewUser> } {
    let users = old.users.map<Nat, OldUser, NewUser>(
      func(_, u) { { u with bio = "" } }
    );
    { users }
  }
}
```

## How Migrations Compose

Migrations form a chain. The compiler verifies each migration's input is compatible with the state produced by all preceding migrations.

| Migration     | Input            | Output                           | Effect                    |
| ------------- | ---------------- | -------------------------------- | ------------------------- |
| `Init`        | `{}`             | `{name : Text; balance : Nat}`  | Initializes both fields   |
| `AddProfile`  | `{}`             | `{profile : Text}`              | Adds a new field          |
| `RenameField` | `{name : Text}`  | `{displayName : Text}`          | Renames name → displayName|

After the full chain: `{displayName : Text; balance : Nat; profile : Text}`. The actor must declare fields compatible with this final state.

## Lifecycle Example: Todo App

Shows how patterns combine across four deployments.

```motoko
// migrations/20250101_000000_Init.mo
module {
  public func migration(_ : {}) : { var nextId : Nat } {
    { var nextId = 0 }
  }
}
```

```motoko
// migrations/20250201_000000_AddTasks.mo
import Map "mo:core/Map";
module {
  type Task = { id : Nat; text : Text; completed : Bool };
  public func migration(_ : {}) : { tasks : Map.Map<Nat, Task> } {
    { tasks = Map.empty<Nat, Task>() }
  }
}
```

```motoko
// migrations/20250301_000000_TaskStatus.mo — transform Bool → variant
import Map "mo:core/Map";
module {
  type OldTask = { id : Nat; text : Text; completed : Bool };
  type NewTask = { id : Nat; text : Text; status : { #pending; #inProgress; #completed } };
  public func migration(old : { tasks : Map.Map<Nat, OldTask> })
    : { tasks : Map.Map<Nat, NewTask> } {
    let tasks = old.tasks.map<Nat, OldTask, NewTask>(
      func(_, task) {
        { id = task.id; text = task.text;
          status = if (task.completed) #completed else #pending }
      }
    );
    { tasks }
  }
}
```

```motoko
// migrations/20250401_000000_AddDueDate.mo — add field to each record
import Map "mo:core/Map";
module {
  type Status = { #pending; #inProgress; #completed };
  type OldTask = { id : Nat; text : Text; status : Status };
  type NewTask = { id : Nat; text : Text; status : Status; due : Int };
  public func migration(old : { tasks : Map.Map<Nat, OldTask> })
    : { tasks : Map.Map<Nat, NewTask> } {
    let tasks = old.tasks.map<Nat, OldTask, NewTask>(
      func(_, task) { { task with due = 0 } }
    );
    { tasks }
  }
}
```

Final state: `{ var nextId : Nat; tasks : Map.Map<Nat, { id : Nat; text : Text; status : { #pending; #inProgress; #completed }; due : Int }> }`

## Runtime Behavior

- On **fresh deploy**: all migrations run in order
- On **upgrade**: only not-yet-applied migrations run (already-applied are skipped)
- **Fast-forward**: safe to skip intermediate deployments — all unapplied migrations run sequentially
- If a migration traps, the upgrade is aborted and the canister stays on the old version

## Compiling

```bash
moc --enhanced-orthogonal-persistence \
    --default-persistent-actors \
    --enhanced-migration=migrations \
    src/main.mo -o main.wasm
```

With mops, split args between global `[moc]` and per-canister `[canisters.backend]`. The migration flag is per-canister because different canisters may use different migration directories:

```toml
[moc]
args = ["--enhanced-orthogonal-persistence", "--default-persistent-actors"]

[canisters.backend]
args = ["--enhanced-migration=migrations"]
```

Then `mops check --fix` and `mops build` work as usual.

## Restrictions

- Cannot combine `--enhanced-migration` with inline `(with migration = ...)`
- Requires enhanced orthogonal persistence
- Actor variables must not have initializers
- Actor body must be static (no top-level side effects except `<system>` calls)
- State after each migration must be compatible with the next migration's input
- Final state must match the actor's declared fields
- Fields in last migration's output not declared in the actor are rejected

## Checklist

- [ ] `migrations/` directory exists next to actor source
- [ ] First migration initializes all fields (`Init.mo` with empty input)
- [ ] Files named with timestamp prefixes for correct ordering
- [ ] Each file exports `public func migration({...}) : {...}`
- [ ] Actor variables declared without initializers
- [ ] `--enhanced-migration=migrations` in `[canisters.backend] args` in `mops.toml`
- [ ] Run `mops check --fix` to verify chain consistency
- [ ] Run `mops build` to compile

## Additional Resources

- Load `writing-motoko` for general Motoko language reference and mo:core APIs
- Load `migrating-motoko` for inline migration without `--enhanced-migration`
