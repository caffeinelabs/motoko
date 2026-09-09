# Persistence

This Motoko build implements a single persistence mode:

* [Enhanced Orthogonal Persistence](OrthogonalPersistence.md):
    This implements scalable persistence with 64-bit main memory that is retained
    across upgrades without stabilization to stable memory. It is the only
    persistence mode; see [classical persistence](OldStableMemory.md) for the
    historical, now removed, 32-bit mode.

Classical (legacy, 32-bit) persistence with Candid-based stabilization was the
original default compilation mode. It was removed in the 1.16 → v2 migration:
`moc` now always targets enhanced orthogonal persistence, and the previously
classical-only flags (`--legacy-persistence`, `--copying-gc`, `--compacting-gc`,
`--generational-gc`, `--rts-stack-pages`) fail with a hard error. Existing
classical canisters are not orphaned: the runtime keeps reading all earlier
classical stable-memory formats, and a classical canister transparently migrates
to enhanced persistence on its next upgrade.

## Compiler Flags

`moc` always uses enhanced orthogonal persistence. The flag
`--enhanced-orthogonal-persistence` is accepted for compatibility and is the
default; the flag `--legacy-persistence` is removed.

The default garbage collector is the incremental GC. The non-incremental
classical GCs (copying, compacting, generational) are removed.

Flags that only apply to enhanced persistence:

Flag              | Applicable Mode
------------------|----------------
--stabilization-instruction-limit | Enhanced persistence only
--stable-memory-access-limit      | Enhanced persistence only

Incremental graph copy stabilization with `__motoko_stabilize_before_upgrade` and `__motoko_destabilize_after_upgrade` is used by enhanced orthogonal persistence and only needed in a seldom case of memory layout upgrade.

## Source Structure

## Runtime System
The Motoko runtime system (RTS) is a combined source base, with a debug and a release build:
* 64-bit enhanced orthogonal persistence, with the incremental GC.

## Compiler
The compiler backend targets enhanced orthogonal persistence:
* `compile-enhanced.ml`: Enhanced orthogonal persistence, 64-bit, passive data segments, incremental graph copy.

The linker integrates the single persistence mode and 64-bit support in one package.

## Tests
Tests apply to the single enhanced-orthogonal-persistence mode. Specific tests
apply to selected subsets, as defined by runner tags.
