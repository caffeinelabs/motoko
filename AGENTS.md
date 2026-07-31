# Agent guide to the Motoko compiler

Orientation for AI agents (code review, coding assistants) working on this
repository. Route to the linked documents instead of guessing; they are the
source of truth.

## What this repository is

The compiler (`moc`) and runtime system for Motoko, a language for the Internet
Computer. The compiler is OCaml, the runtime system is Rust compiled to Wasm.
The base/core libraries are NOT in-tree (they live in the external
`motoko-core` / `motoko-base` repositories); only compiler-known stubs are
vendored under `test/*-stub`.

## Repository map

| Area | Path | Notes |
|---|---|---|
| Compiler source | `src/` | OCaml; per-library subdirectories, see [src/Structure.md](src/Structure.md) |
| Frontend (parse, typecheck) | `src/mo_frontend/` | accepted/rejected programs change here |
| AST / types / values | `src/mo_def/`, `src/mo_types/`, `src/mo_values/` | |
| IR + passes | `src/ir_def/`, `src/ir_passes/`, `src/lowering/` | |
| Interpreters | `src/mo_interpreter/`, `src/ir_interpreter/` | reference semantics (`moc -r`); must agree with codegen — a semantic change in one path usually needs mirroring in the others |
| Codegen (Wasm emission) | `src/codegen/`, `src/wasm-exts/` | changes affect emitted runtime behavior |
| Pipeline / CLI flags | `src/pipeline/`, `src/mo_config/` | |
| Prelude / `Prim` | `src/prelude/` | alters observable language semantics |
| Error codes | `src/lang_utils/error_codes.ml` + `src/lang_utils/error_codes/M0NNN.md` | append-only registry |
| Runtime system | `rts/` | Rust crates (`motoko-rts*`) producing `mo-rts*.wasm`; GC, layout, stable memory |
| Tests | `test/` | see [test/README.md](test/README.md) |
| Design docs | `design/` | e.g. [OrthogonalPersistence.md](design/OrthogonalPersistence.md), [Stable.md](design/Stable.md), [Implementation.md](design/Implementation.md) |
| User docs | `doc/` | legacy `doc/md/`, Starlight site `doc/site/` |
| Build | `flake.nix`, `nix/`, `src/Makefile`, `src/dune*` | see [Building.md](Building.md) |
| CI | `.github/workflows/` | see [CI.md](CI.md) |

## Conventions that reviews and changes must respect

- **OCaml style**: [src/Conventions.md](src/Conventions.md). Warnings policy:
  there shall be none. `assert false` only for genuinely impossible cases —
  a new `assert false`/`failwith`/non-exhaustive `match` reachable from valid
  Motoko input is a bug.
- **Tests are expectation-based**: a test `foo.mo` pairs with `ok/foo.*.ok`
  files regenerated via `make accept` (or `run-test -a foo.mo`). Changing
  compiler output without updating the paired `.ok` files (or vice versa) is a
  defect. Silent tests produce no output and have no `.ok` by design.
  Directories encode the kind: `run/` (interpret+wasm), `run-drun/` (IC
  semantics), `fail/` (must not typecheck, expectations in `fail/ok/*.tc.ok`),
  plus `perf/`, `repl/`, `mo-idl/`, etc.
- **New warnings/errors** need an `M0NNN` code registered in
  `src/lang_utils/error_codes.ml` (append, never renumber) and test coverage
  of the new diagnostic.
- **Changelog.md**: user-visible language/`moc`/`mo-doc` changes get an entry
  at the top — under the `## Next` heading when present (create it right after
  a release if absent), always above the most recent `## X.Y.Z (date)` section —
  ending with the PR number `(#NNNN)`. Released sections are frozen history —
  never edit them. Internal-only refactors need no entry.
- **Compatibility**: renames/removals of public compiler surface (CLI flags,
  `src/prelude/` primitives, stable-signature semantics) need a
  deprecation/migration path; see `design/` for persistence and stability
  invariants.

## Building and testing

Environment via Nix: `nix develop`, then `make -C src moc` to build and
`make -C test` (or `run-test test/run/foo.mo`) to test — details in
[Building.md](Building.md) and [test/README.md](test/README.md). CI must be
green; replicate locally with `nix build --no-link`.

## High-risk areas (extra scrutiny)

Changes here can miscompile programs or corrupt canister state even when all
tests pass, because coverage is necessarily incomplete:

- `src/codegen/**`, `src/wasm-exts/**` — Wasm emission.
- `rts/**` — GC, heap layout, stable memory, IC system API.
- `src/mo_types/**` typing rules — soundness; a wrongly-accepted program is
  worse than a wrongly-rejected one.
- Interpreter/codegen divergence — a semantic change landed in only one of
  `src/mo_interpreter/`, `src/ir_interpreter/`, `src/codegen/` silently splits
  `moc -r` behavior from compiled Wasm.
- Stable compatibility / orthogonal persistence (`design/Stable.md`,
  `design/OrthogonalPersistence.md`) — upgrade-safety invariants.
