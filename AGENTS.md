# AGENTS.md

Motoko is a compiler and runtime for the Motoko programming language, which targets
WebAssembly canister smart contracts on the Internet Computer.

## Environment

The build is driven by [Nix](https://nixos.org/) flakes. Enter the dev shell with
`nix develop`, or use [direnv](https://direnv.net/) + nix-direnv (`.envrc` runs
`use flake`). All commands below assume you are inside this shell; outside it you
will hit errors like `library not found for -lm` or missing `dune`/`cargo`.

Requires Nix with `nix-command` and `flakes` experimental features enabled. Pinned
tool versions come from `flake.nix`/`flake.lock`; do not edit `flake.lock` by hand.

## Build

The OCaml sources live in `src/`, which is also the dune project root.

- `make -C src` — build all binaries (`moc`, `mo-ld`, `mo-doc`, `didc`, JS targets).
- `make -C src moc` — build just the `moc` binary.
- `dune build --root src` — equivalent full build; `src/moc` is a symlink to
  `_build/default/exes/moc.exe`.
- `make -C rts` — build the Motoko runtime system (Rust + C).

## Test

Run from the repository root.

- `make -C test` (or `make -C src test`) — run the full test suite.
- `make -C test quick` / `make -C test parallel` — quick or parallel subsets.
- `make -C src unit-tests` — dune inline tests (`unit-tests-accept` to promote).
- `nix build --no-link` — build and test everything the way CI does.

Tests live under `test/`, grouped by category (`run/`, `run-drun/`, `fail/`,
`trap/`, ...). Expected output lives in each category's `ok/` subdirectory; actual
output goes to gitignored `_out/`. Update expected output only via the accept
mechanism (`make ... accept` or the runner's `-a` flag), never by hand.

## Format

Formatting is CI-enforced; a mismatch fails the build.

- `make -C src format` — OCaml formatting (`ocamlformat`, only `src/docs/*.{ml,mli}`
  and `src/exes/deser.ml` are checked).
- `make -C rts format` — Rust formatting (`cargo fmt` over the `motoko-rts*` crates).

## Generated files — never hand-edit

- `doc/md/examples/grammar.txt` is generated from `src/mo_frontend/parser.mly`.
  Regenerate with `make -C src grammar`; CI fails if it is stale.
- Anything under `_build/`, `_out/`, `result*` (build outputs, gitignored).

## Directory map

- `src/` — the compiler (OCaml); see `src/Structure.md` for the module layout.
- `rts/` — runtime system (Rust `motoko-rts*` crates + C/libtommath).
- `test/` — the test suite; `test-runner/` is the Rust harness that runs it.
- `doc/` — documentation sources and the website (`doc/site/`).
- `design/` — design notes and language/implementation specs.
- `spec/` — the LaTeX formal specification.
- `nix/` — Nix derivations for builds, releases, and CI checks.
- `bin/` — wrapper scripts and symlinks to built tools.
- `wasm-profiler/` — Rust tool for profiling generated Wasm.

## Conventions

- `.mo` files are classified as Swift for syntax highlighting (`.gitattributes`).
- Error codes referenced in `fail/` tests must exist in
  `src/lang_utils/error_codes.ml`; CI (`test/check-error-codes.py`) verifies this.
- `Changelog.md` must keep its `## X.Y.Z (YYYY-MM-DD)` heading format at the top;
  the release tooling parses it.
