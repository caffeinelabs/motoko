---
name: migrating-motoko
description: >-
  Inline actor migration with (with migration = ...). Use when upgrading
  canister state, changing field types, or writing migration functions
  without the --enhanced-migration flag.
---

# Inline Actor Migration

Migrate actor state across canister upgrades using a migration expression attached to the actor. Each upgrade has at most one migration function.

**For multi-migration with a `migrations/` directory**, load `migrating-motoko-enhanced` instead.

## When to Use

### Implicit migration (no code needed)

The runtime allows the upgrade if the new program is compatible with the old:

- Adding actor fields
- Removing actor fields
- Changing mutability (`var` ↔ `let`)
- Adding variant constructors
- Widening types (`Nat` → `Int`)

### Explicit migration required

- Renaming fields
- Changing a field's type (e.g. `Bool` → variant, `Int` → `Float`)
- Restructuring state (splitting/merging fields)
- Transforming collection values

## Syntax

Parenthetical expression immediately before the actor:

```motoko
import Migration "migration";

(with migration = Migration.run)
actor {
  var newState : Float = 0.0;
};
```

Or inline:

```motoko
import Int "mo:core/Int";

(with migration = func(old : { var state : Int }) : { var newState : Float } {
  { var newState = old.state.toFloat() }
})
actor {
  var newState : Float = 0.0;
};
```

Or using the shorthand when the imported module exports a `migration` field:

```motoko
import { migration } "migration";

(with migration)
actor { ... };
```

## Migration Function Rules

- Type: `func (old : { ... }) : { ... }` — local, non-generic, both records must use persistable types (no functions or mutable arrays)
- **Domain**: old actor fields (names and types from the previous version)
- **Codomain**: new actor fields (must exist in the new actor with compatible types)
- Runs **only on upgrade** — on fresh install, initializers run normally
- If the migration traps, the upgrade is aborted and the canister stays on the old version

### Field semantics

| Field appears in | Effect |
| ---------------- | ------ |
| Input and output | Field is transformed |
| Output only      | New field produced by migration |
| Input only       | Field consumed (compiler warns about possible data loss) |
| Neither          | Carried through or initialized by declaration |

## Migration Module Pattern

Keep migrations in a separate module. Define old types inline — do not import them from old code paths:

```motoko
// migration.mo
import Types "types";
import Map "mo:core/Map";

module {
  type OldTask = { id : Nat; title : Text; completed : Bool };

  type OldActor = {
    var tasks : Map.Map<Nat, OldTask>;
    var nextId : Nat;
  };

  type NewActor = {
    var tasks : Map.Map<Nat, Types.Task>;
    var nextId : Nat;
  };

  public func run(old : OldActor) : NewActor {
    let tasks = old.tasks.map<Nat, OldTask, Types.Task>(
      func(_, task) {
        {
          id = task.id;
          title = task.title;
          due = 0;
          var status = if (task.completed) #completed else #pending;
        }
      }
    );
    { var tasks; var nextId = old.nextId };
  };
};
```

```motoko
// main.mo
import Migration "migration";

(with migration = Migration.run)
actor {
  var tasks : Map.Map<Nat, Types.Task>;
  var nextId : Nat = 0;
};
```

## Common Patterns

### Add field with default

```motoko
old.users.map<Nat, OldUser, NewUser>(
  func(_, u) { { u with zipCode = "" } }
)
```

### Add optional field

```motoko
{ task with var assignee = null : ?Principal }
```

### Bool to variant

```motoko
var status = if (task.completed) #completed else #pending;
```

### Rename a field

Consume old name, produce new name:

```motoko
func(old : { var state : Int }) : { var value : Int } {
  { var value = old.state }
}
```

### Drop a field

Consume it in the input, omit from output. Compiler warns — ensure the loss is intentional.

## Checklist

- [ ] Decide: implicit (compatible change) or explicit (migration function)
- [ ] If explicit: define old types inline in `migration.mo`
- [ ] Migration type: `func (old : RecordIn) : RecordOut` with persistable types
- [ ] Attach with `(with migration = Migration.run)` before the actor
- [ ] Do not use `preupgrade`/`postupgrade` for data migration
- [ ] Verify with `mops check --fix` and `mops build`

## Additional Resources

- Load `writing-motoko` for general Motoko language reference
- Load `migrating-motoko-enhanced` for multi-migration with `--enhanced-migration`
