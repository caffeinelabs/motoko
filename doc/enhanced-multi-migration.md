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

1. **`type_1, type_2, ..., type_n`** (`intermediate_types`) — precise Memory
   object type at each migration boundary. Computed by **reverse-folding** from
   `type_n` (= actor's `mem_ty`), undoing each migration from last to first.
   For each step, range-only fields are removed (introduced by that migration),
   domain fields are restored with their domain types, and everything else is
   carried through. This mirrors the `stab_fields_pre` calculation in the old
   `(with migration = fn)` syntax: `type_k` contains exactly the fields that
   exist at boundary `k`, with no ghost fields from dropped-and-never-reintroduced
   fields.

2. **`type_0`** — base case for `ICStableRead` when no migration has been performed
   yet. Uses Init's domain type (for pre-migration actor adoption). On fresh
   install, `ICStableRead` returns defaults (all `?T` fields are `None`).

### Generated IR (runtime)

The generated IR is a nested if-expression. Each level `k` is strongly typed
at `type_k`.

```
let state_n : type_n =
  if was_migration_performed(m_{n-1}):
    ICStableRead(type_n)                          // already at boundary n
  else:
    let state_{n-1} : type_{n-1} =
      if was_migration_performed(m_{n-2}):
        ICStableRead(type_{n-1})                  // already at boundary n-1
      else:
        ...
          let state_0 : type_0 =
            ICStableRead(type_0)                  // fresh install or pre-migration
          let input_0  = extract dom fields from state_0
          let output_0 = m_0.run(input_0)
          register_migration(m_0)
          merge(output_0, state_0) -> state_1     // range fields wrapped in ?T,
        ...                                       // carried fields from state_{k-1}
    let input_{n-1}  = extract dom fields from state_{n-1}
    let output_{n-1} = m_{n-1}.run(input_{n-1})
    register_migration(m_{n-1})
    merge(output_{n-1}, state_{n-1}) -> state_n

ICStableStore(mem_ty)                             // update stored metadata
result = project state_n to actor's mem_ty        // drop fields actor no longer declares
```

Each `merge(output_k, state_{k-1})` builds `type_k` by:
- Taking range fields from `output_k` (wrapped in `?T`)
- Carrying non-range fields from `state_{k-1}` unchanged

Each `extract` unwraps `?T` fields from the previous state, trapping if a
required domain field is `None` (should never happen in valid programs).

## RTS Compatibility Check

**Problem:** Passing a single type to `ICStableRead` traps when fields change
type across deployments — the stored type has the old field type, the expected
type has the new one.

**Solution:** The nested if-expression checks `was_migration_performed` from the
last migration inward. The outermost hit determines boundary `k`, and that branch
calls `ICStableRead(type_k)`. Because `type_k` is computed by reverse-folding from
the actor's type, it precisely matches what was stored at boundary `k` — no ghost
fields, correct types. The RTS check allows new optional fields and verifies
compatible types for matching fields. On fresh install the stored type is default,
so the check is skipped automatically.

After the nested expression, `ICStableStore(mem_ty)` updates the stored metadata
to the actor's actual declared fields. This ensures the next deployment's check
sees the correct type.

## Key Properties

- **Idempotency.** Already-applied migrations are skipped; redeploying is a no-op.
- **Fast-forward.** v1 → v100 directly, or v1 → v50 → v100, or step-by-step.
- **Type changes.** Each level is strongly typed at `type_k`; type changes are
  handled naturally by the migration's `run` function.
- **Field drops.** Remove the field from the actor declaration. No migration
  needed. The reverse-fold removes such fields from earlier types; the final
  projection drops fields the actor no longer declares.
- **Partial migrations.** Each migration only mentions the fields it transforms.
  Unmentioned fields are carried through from the previous state (same as old syntax).
- **Init migration required.** First deployment needs an init migration
  (`{} -> { initial fields }`).
- **Split deployments safe.** `ICStableRead(type_k)` checks against the precise
  boundary type; `ICStableStore(mem_ty)` updates metadata for the next deployment.

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
