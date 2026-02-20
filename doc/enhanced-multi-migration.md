# Enhanced Multi-Migration (`--enhanced-migration <dir>`)

## Overview

The compiler reads all `.mo` migration modules from the specified directory, sorts
them lexicographically (timestamp-prefixed by convention), and compiles them into
the actor as a migration chain. Each module exposes a
`public func run(old : {...}) : {...}` that transforms a subset of stable fields.

## Compile-Time Validation (`typing.ml`)

The validation is designed to give each incremental step `v_i → m_{i+1} → v_{i+1}`
the same semantics as the old `(with migration = fn)` syntax: each migration only
needs to mention fields it transforms; everything else is retained from the
accumulated state.

### Per-migration checks

- Each `run` must be a local function with stable object input/output types.
- Mutually exclusive with the old `(with migration = fn)` syntax.

### Accumulated composition

Given migration chain `[m_0, m_1, ..., m_n]`:

For **every** k from 1 to n, the compiler checks
`accumulated(m_0 .. m_{k-1}) <: input(m_k)`.
This is necessary because each migration has access
to the full accumulated state (via merge semantics), not just the previous
migration's output. A migration can reference a field produced by any earlier
migration, not just the immediately preceding one.

The accumulated type is built incrementally by last-writer-wins over each
migration's output (range). Fields not overwritten by later migrations persist
from earlier ones. The first migration (k=0) is skipped — it has no predecessor.

### Last migration vs actor (mirrors old syntax)

The last migration `m_n` is checked against the current actor with the same
semantics as `(with migration = fn)`. The accumulated state of `m_0 .. m_{n-1}`
plays the role of the "stored heap state":

- For each actor stable field `f`:
  - If `f` is in `m_n`'s range → check type compatibility.
  - If `f` is not in `m_n`'s range but in `accumulated_pre` → **retained**;
    check type compatibility against the accumulated pre-state type.
    (This is strictly more precise than the old syntax, which cannot type-check
    retained fields at compile time.)
  - If `f` is in neither → **error**: the chain does not produce this field.
- **M0205**: fields in `m_n`'s range not declared in the actor → error.
- **M0206**: `m_n` consumes a field but doesn't produce it, field is in actor → warning.
- **M0207**: `m_n` consumes a field but doesn't produce it, field not in actor → warning (data loss).

Fields from earlier migrations that are not in the current actor are silently
tolerated — they are historical artifacts from field drops in earlier versions.

## Upgrade-Time Algorithm (`desugar.ml`)

Given migration chain `[m_0, m_1, ..., m_{n-1}]`:

### Type computation (compile-time)

1. **`type_1, type_2, ..., type_n`** (`intermediate_types`) — accumulated Memory
   object type after each migration boundary. Built by scanning migrations in
   order and applying last-writer-wins over each migration's input/output fields.
   `type_k` represents the stable type that would be stored on the heap if exactly
   k migrations had been applied. Used to select the correct `ICStableRead` type
   at runtime.

2. **`enhanced_mem_ty`** — `type_n` extended with the actor's declared stable
   fields (`stab_fields`) merged last (last-writer-wins). In valid programs this
   equals `type_n` (typing.ml ensures the final migration output covers all actor
   fields), but the explicit merge acts as a safety net for the fresh-install
   fallback (`type_at(0)`), guaranteeing the load type always includes the
   actor's declared fields.

3. **`any_enhanced_mem_ty`** — same fields as `enhanced_mem_ty` but all typed
   `?Any`. Used as the type of the mutable state accumulator during migration
   execution. A uniform type is needed because migrations can change field types;
   `?Any` accommodates any value at any point in the chain.

### Generated IR (runtime)

```
// Phase 1: find boundary + check + load (backward if-else)
if was_migration_performed(m_{n-1}):
  state = ICStableRead(type_n)                      // check + load
  acc = cast type_n fields to ?Any, null for rest
elif was_migration_performed(m_{n-2}):
  state = ICStableRead(type_{n-1})
  acc = cast type_{n-1} fields to ?Any, null for rest
...
else:
  state = ICStableRead(enhanced_mem_ty)             // fresh install
  acc = cast all fields to ?Any

// Phase 2: execute migrations sequentially (mutable state)
var state = acc;
for each m_i in [m_0 .. m_{n-1}]:
  if not was_migration_performed(m_i):
    input  = cast relevant ?Any fields to m_i's input type  // CastPrim
    output = m_i.run(input)
    state := merge output back to ?Any, carry forward rest  // CastPrim
    register_migration(m_i)

// Phase 3: finalize
ICStableStore(mem_ty)                          // ICStableStore
result = cast ?Any fields to actor's declared types // CastPrim
         (fields not declared by actor are dropped)
```

### Generated IR (runtime) (alternative)

```
type_n state_{n} =
if was_migration_performed(m_{n-1}):
  ICStableRead(type_n)                      // check + load
elif was_migration_performed(m_{n-2}):
  { state_{n-1} =
      if was_migration_performed(m_{n-3}) ...
          .. type_1 state_1 =
	       if wasm_migration_performed(m_0)
	         ICStableRead(type_1)
	       elif {
	         let state_0 = {}
	         let output = m_{0}.run(state_0)
		 register_migration(m_0);
		 merge(output, state_0)
	       };
    output = m_{n-1}.run(state_{n-1})
    register_migration(m_{n-1});
    merge(output, state_{n-1} }
ICStableStore(mem_ty)                          // ICStableStore
result = state_{n};
```

### Why `?Any` for the state

The mutable state variable must have a fixed type that accommodates all possible
field types across the migration chain. When a migration changes a field's type
(e.g., `Nat` → `[Nat]`), the state needs to hold both the old and new types at
different points. `?Any` is the uniform type that satisfies this. `CastPrim`
(a Wasm no-op) converts between `?Any` and concrete types at extraction/merge
boundaries.

## RTS Compatibility Check

**Problem:** Passing `enhanced_mem_ty` directly to `ICStableRead` traps
when fields change type across deployments — the stored type has the old field
type, `enhanced_mem_ty` has the new one.

**Solution:** A backward if-else chain checks `was_migration_performed` from the
last migration downward. The first hit determines boundary `k`, and that branch
calls `ICStableRead(type_k)`. The `register_stable_type` check inside sees
`type_k` (the accumulated type at boundary `k`) which is a superset of the
previous deployment's stored type. The RTS check allows new fields and verifies
compatible types for matching fields. On fresh install the stored type is default,
so the check is skipped automatically.

Each branch loads with only `type_k` fields, then expands to `any_enhanced_mem_ty`
by casting loaded fields to `?Any` and setting missing fields to null.

After the fold, `assign_stable_type(mem_ty)` updates the stored metadata to
the actor's actual declared fields. This ensures the next deployment's check
sees the correct type, and no ghost fields persist in the metadata.

## Key Properties

- **Idempotency.** Already-applied migrations are skipped; redeploying is a no-op.
- **Fast-forward.** v1 → v100 directly, or v1 → v50 → v100, or step-by-step.
- **Type changes.** `CastPrim` + `?Any` handles field type changes transparently.
- **Field drops.** Remove the field from the actor declaration. No migration
  needed. The field persists as a ghost in the accumulated state but is projected
  away at the end. Re-introduction by a later migration creates the field fresh.
- **Partial migrations.** Each migration only mentions the fields it transforms.
  Unmentioned fields are retained from the accumulated state (same as old syntax).
- **Init migration required.** First deployment needs an init migration
  (`{} -> { initial fields }`).
- **Split deployments safe.** `ICStableRead(type_k)` checks against the correct
  boundary type; `assign_stable_type(mem_ty)` updates metadata for the next deployment.

### Example: full lifecycle

```
// v0: Init
run : {} -> {a : Nat}                     actor { stable let a : Nat }

// v1: Add field b
run : {} -> {b : Nat}                     actor { stable let a : Nat; stable let b : Int }
  // accumulated_pre = {a : Nat}; b : Nat <: Int ✓; a retained from pre ✓

// v2: Change b's type
run : {b : Int} -> {b : Bool}             actor { stable let a : Nat; stable let b : Bool }
  // accumulated_pre = {a : Nat, b : Nat}; Nat <: Int for input ✓; a retained ✓

// v3: Drop a
run : {a : Nat} -> {}                     actor { stable let b : Bool }
  // accumulated_pre = {a : Nat, b : Bool}; b retained ✓; a consumed, not produced → M0207 warning

// v4: Reintroduce a with new type
run : {} -> {a : Text}                    actor { stable let a : Text; stable let b : Bool }
  // accumulated_pre = {a : Nat, b : Bool}; a : Text from m_4 ✓; b retained ✓
```

## Usage

```bash
moc --enhanced-orthogonal-persistence \
    --enhanced-migration ./migrations \
    actor.mo -o actor.wasm
```

Migration directory convention:
```
migrations/
  20250101_000000_Init.mo          # {} -> { initial fields }
  20250115_120000_AddProfile.mo    # { ... } -> { ... + profile fields }
  20250201_090000_ChangeTypes.mo   # { x : Text, ... } -> { x : [Text], ... }
```
