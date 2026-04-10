---
name: build-and-test
description: Build the Motoko compiler (moc) and run tests in the motoko repo. Use when the user asks to build, compile, rebuild, run tests, run test-runner, or verify changes to OCaml source files.
---

# Build and Test — Motoko Compiler

## Environment Setup

The repo uses [nix-direnv](https://github.com/nix-community/nix-direnv) to
automatically load the nix dev environment. The `.envrc` at the repo root calls
`use flake`, and nix-direnv caches the result in `.direnv/`.

### Setup snippet (run once per shell session)

```bash
cd "$(git rev-parse --show-toplevel)"
eval "$(direnv export bash 2>/dev/null)"
unalias moc 2>/dev/null
```

This navigates to the repo root (works from any subdirectory or worktree),
then loads the cached nix environment (~500ms, instant after first run). PATH is
set so GNU tools (`paste`, `sed`, `grep`) take precedence over macOS system tools,
and all project env vars (`MOTOKO_BASE`, `MOTOKO_CORE`, etc.) are set. Shell state
persists across subsequent calls, so this only needs to run once.

**Important**: The `unalias moc` is still needed. The user's shell may have
`moc` aliased to a mops-installed version (e.g. `moc=$(mops toolchain bin moc)`).
In zsh, **aliases take precedence over PATH**, so even with the correct PATH,
`run.sh` would silently use the wrong `moc`. Always verify with `which moc` —
it should print the local `bin/moc` or `src/moc` path, not a mops path.

**Important**: Stay in the repo root. Both `test-runner` and `dune build --root src`
expect to be run from there. `test-runner` will fail silently if `test/run.sh` is
not found relative to cwd.

## Building

The dune project root is `src/`, not the repo root.

```bash
# Full build — all native exes, JS targets, and inline tests (recommended)
dune build --root src

# Fast build — only the moc native binary
dune build --root src exes/moc.exe
```

Use `dune build` (all targets) to catch errors in JS targets (`moc.js`,
`moc_interpreter.js`, `didc.js`) and other executables (`mo-ld`, `didc`, etc.)
that `dune build exes/moc.exe` would miss.

`src/moc` is a symlink to `_build/default/exes/moc.exe`.

The user typically runs `dune build -w` in a separate terminal for continuous rebuilds.
If the user says they have a watch build running, skip the build step — `src/moc` is
already up to date.

## Running Tests

Tests live in `test/` with subdirectories per category: `fail/`, `run/`, `run-drun/`,
`trap/`, etc. Use `test-runner` (a Rust tool in the nix shell) to run tests. It wraps
`test/run.sh`, runs tests in parallel (8 threads), and automatically applies the
correct flags per test category (`-t` for fail, `-d` for run-drun, etc.).

**All `test-runner` commands must be run from the repo root. Always pass `-b`
for batch mode** (the interactive picker doesn't work for agents).

### Examples

```bash
test-runner -baf fail/             # all fail tests + accept changes (~25s)
test-runner -baf contextual-dot    # word match — all tests with "contextual-dot" in the path
test-runner -baf "lambda.*"        # regex match
test-runner -bf fail/              # run without accepting (check for diffs)
```

### Flags

| Flag | Purpose |
|------|---------|
| `-a` | Accept: update `ok/` files with actual output |
| `-f <pattern>` | Filter tests by name (word match or regex) |
| `--in-file` | Match filter against test *output* file contents instead of names |
| `--just-tc` | Typecheck only (overrides per-category defaults) |

Never update expected output manually — always use `-a` to accept changes.

**Avoid running full `run/` or `run-drun/` suites locally** — they take close to
an hour. The full `fail/` suite is fine (~25 seconds). For `run/` and `run-drun/`,
run only the specific tests relevant to your change:

```bash
test-runner -baf some-test         # targeted by name pattern
```

## Test File Conventions

- `//MOC-FLAG --some-flag`: extra flags passed to `moc`
- `//MOC-ENV VAR=value`: environment variables for the test
- `//SKIP ext`: skip a specific output extension
- Expected outputs go in `ok/` subdirectory as `testname.ext.ok`
- Actual outputs go in `_out/` (gitignored)

## Troubleshooting

**`test-runner` exits with code 1 and no output**:
You are not in the repo root. `test-runner` checks for `test/run.sh` relative
to cwd and exits silently if it's missing. Run `cd "$(git rev-parse --show-toplevel)"`
first.

**`paste: illegal option` or `sed: first RE may not be empty`**:
Nix environment not loaded. Run the setup snippet above. If `direnv export`
fails, check that `direnv allow` was run in the repo root.

**`library not found for -lm`**:
Nix environment not loaded. Run the setup snippet above.

**`moc: target src/moc not built yet`**:
Run `dune build --root src exes/moc.exe`.

**Wrong `moc` version (from mops/system)**:
Run `unalias moc 2>/dev/null`. In zsh, aliases shadow PATH lookups, so a mops alias
will win even with the correct PATH. Verify with `which moc` — it should show the
local `bin/moc` or `src/moc` path, not a mops path.
