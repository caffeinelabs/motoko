# Enhanced Multi-Migration (`--enhanced-migration <dir>`)

## Overview

The enhanced multi-migration feature allows canister developers to define a chain of
incremental migration steps as separate Motoko modules. The compiler reads all `.mo`
migration modules from the specified directory, sorts them lexicographically (by
convention, files are timestamp-prefixed to ensure deterministic ordering), and
compiles them into the actor as a migration chain.

Each migration module exposes a `public func run(old : {...}) : {...}` that consumes
a subset of the actor's stable fields and produces a (possibly different) subset.

## Compile-Time Validation (`typing.ml`)

- Each migration's `run` function must have stable object input/output types.
- The chain composes: each migration's output is a subtype of the next migration's
  input (`output[i] <: input[i+1]`).
- The final migration's output must cover all of the actor's declared stable fields.
- Enhanced migrations and the old `(with migration = fn)` syntax are mutually
  exclusive.

## IR Generation (`desugar.ml`)

The desugaring phase transforms the validated migration chain into IR code that
executes the migrations at upgrade time. The pipeline has five stages:

### 1. Read the Old State

The compiler computes `enhanced_mem_ty` — the union of every field mentioned across
all migrations and the actor, with last-writer-wins semantics for types — and passes
it to `ICStableRead` to load the old actor's persistent heap object.

The RTS compatibility check is skipped (`assign_stable_type` instead of
`register_stable_type`) because the compile-time chain validation already ensures
correctness and the standard check would otherwise reject valid type-changing
migrations. See [RTS Compatibility Check](#rts-compatibility-check) for details.

### 2. Cast to Uniform Accumulator

Because field types may change across migrations (e.g., `Text` → `[Text]`), all
fields are immediately cast via `CastPrim` (a representational no-op at Wasm level)
into a uniform `?Any` accumulator type (`any_enhanced_mem_ty`).

This avoids IR type-checking (`Check_ir`) failures when the same field name carries
different types at different stages of the chain. `CastPrim` satisfies `Check_ir`'s
rule — `typ(e) <: t1` and `t2 <: expected` — without imposing any relationship
between `t1` and `t2`, effectively allowing arbitrary type reinterpretation at the IR
level while compiling to a no-op at Wasm level.

### 3. Fold Through Migrations

For each migration in order:

- **Skip check:** Query `was_migration_performed` (RTS primitive) to determine if
  this migration has already been applied. If so, skip it entirely.
- **Extract:** Cast the relevant `?Any` fields from the accumulator to the
  migration's concrete input types via `CastPrim`.
- **Run:** Call the migration's `run` function.
- **Merge:** Cast the output fields back to `?Any` and merge into the accumulator.
  Fields not produced by the migration pass through unchanged.
- **Register:** Call `register_migration` (RTS primitive) to mark the migration as
  applied.

### 4. Final Projection

After the fold completes, cast each `?Any` field to the actor's declared concrete
type via `CastPrim` and construct the actor's `mem_ty` object. Fields present in the
accumulator but not declared by the actor are silently dropped at this step.

### 5. Actor Initialization

The projected result becomes the actor's initial stable state. The `ICStableWrite`
during the next `preupgrade` stores the actor's `mem_ty` (the concrete final type)
via `assign_stable_type`.

## RTS Compatibility Check

The standard `register_stable_type` RTS function checks that the new stable type is
memory-compatible with the previously stored type before allowing the upgrade. For
enhanced migrations, this check is counterproductive:

- `enhanced_mem_ty` uses last-writer-wins for field types, so if any migration
  changes a field's type (e.g., `x : ?Text` → `x : ?[Text]`), the new type will
  differ from the stored old type.
- The RTS would trap with "Memory-incompatible program upgrade" before the migration
  code gets a chance to run.
- The compile-time chain validation already ensures the migration chain is
  well-formed.
- The IC provides atomic rollback: if the upgrade traps for any reason, the canister
  state reverts entirely.

Therefore, the enhanced migration path uses `assign_stable_type` (no compatibility
check) instead of `register_stable_type`. This is signaled via a boolean flag on the
`ICStableRead` IR primitive.

## Key Properties

- **Idempotency.** Already-applied migrations are skipped via the RTS registry, so
  redeploying the same binary is a no-op.
- **Fast-forward.** Upgrading directly from version 1 to version N produces the same
  result as upgrading step-by-step through each intermediate version, because the
  migration chain is deterministic, each step's types are validated, and the final
  projection only keeps the actor's declared fields.
- **Type changes allowed.** Fields may change type across migrations. The
  `CastPrim` + `?Any` accumulator strategy handles this transparently without
  requiring intermediate IR types to be consistent.
- **Field lifecycle.** Fields are carried through the `?Any` accumulator even if an
  intermediate migration doesn't produce them. They are only truly "dropped" at the
  final projection step if the actor's current stable fields do not declare them.
- **Init migration required.** The first deployment requires an "init" migration with
  empty input (`{}`) that provides initial values for all stable fields.

## Migration Directory Convention

```
migrations/
  20250101_000000_Init.mo          # {} -> { initial fields }
  20250115_120000_AddProfile.mo    # { ... } -> { ... + profile fields }
  20250201_090000_ChangeTypes.mo   # { x : Text, ... } -> { x : [Text], ... }
```

Files are sorted lexicographically. Timestamp prefixes ensure correct ordering and
prevent out-of-order insertion. Never rename or modify already-deployed migration
files.

## Usage

```bash
moc --enhanced-orthogonal-persistence \
    --enhanced-migration ./migrations \
    actor.mo -o actor.wasm
```

## Implementation Files

| File | Role |
|------|------|
| `src/mo_frontend/typing.ml` | Compile-time chain validation |
| `src/lowering/desugar.ml` | IR generation (CastPrim + ?Any pipeline) |
| `src/ir_def/ir.ml` | `ICStableRead` primitive definition |
| `src/codegen/compile_enhanced.ml` | Wasm codegen, `assign_stable_type` dispatch |
| `rts/motoko-rts/src/persistence.rs` | RTS `register_stable_type` / `assign_stable_type` |