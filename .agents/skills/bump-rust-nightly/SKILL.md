# Bumping the Rust Nightly Toolchain

This skill guides you through bumping the `rustc-nightly` version used to build
the Motoko RTS.

## Files to Change

| File | What to update |
|------|----------------|
| `nix/pkgs.nix` | Nightly date string in `rust-bin.nightly."YYYY-MM-DD"` |
| `nix/rts.nix` | `rustStdDepsHash` (fixed-output derivation hash) |
| `flake.lock` | Run `nix flake update rust-overlay` to get latest overlay |
| `rts/motoko-rts/Cargo.toml` | Bump any exact-pinned crate versions if vendoring fails |
| `rts/motoko-rts/Cargo.lock` | Run `cargo update` in `rts/motoko-rts/` |
| `rts/motoko-rts-tests/Cargo.lock` | Run `cargo update` in `rts/motoko-rts-tests/` |

## Step-by-Step

### 1. Switch branch and rebase
```sh
git checkout gabor/bump-nightly-rustc
git rebase origin/master
```

### 2. Update rust-overlay
```sh
cd ~/motoko && nix flake update rust-overlay
```

### 3. Set nightly date in `nix/pkgs.nix`
```nix
rust-nightly = self.rust-bin.nightly."YYYY-MM-DD".default.override {
```
Start with a recent date (e.g. a few weeks before today). If tests hang or
fail, bisect backwards by 2-4 weeks at a time.

### 4. Get new `rustStdDepsHash`
Set it to a dummy value first:
```nix
rustStdDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
```
Then run:
```sh
nix build .#rts 2>&1 | grep "got:"
```
Paste the `got:` hash back into `nix/rts.nix`. Note: this hash is often
**identical across several months** of nightly dates.

### 5. Update Cargo lockfiles
```sh
cd rts/motoko-rts       && cargo update
cd rts/motoko-rts-tests && cargo update
```
If `nix build .#rts` fails with "failed to select version for `foo = =X.Y.Z`",
bump that exact pin in `rts/motoko-rts/Cargo.toml` and re-run `cargo update foo`.

### 6. Build and test
```sh
nix build .#rts
```
Watch for compiler errors — common ones encountered and their fixes:

#### `.json target specs require -Zjson-target-spec`
Add `-Zjson-target-spec` to `NIGHTLY_CARGO_OPTIONS` in `rts/Makefile`.

#### `target-pointer-width: invalid type: string "32", expected u16`
Change `"target-pointer-width": "32"` → `"target-pointer-width": 32` (integer)
in `rts/motoko-rts/wasm32-none-shared.json` and `wasm64-none-shared.json`.

#### `panic_immediate_abort is now a real panic strategy`
Remove `panic_immediate_abort` from `-Zbuild-std-features` and add
`-Zunstable-options -Cpanic=immediate-abort` to `RTS_RELEASE_COMPILE_FLAGS`.
Use `NIGHTLY_RELEASE_CARGO_OPTIONS` in `rts/Makefile`.

#### `failed to select version for rustc-literal-escaper = "^0.0.7"`
The vendored std deps are stale — re-run the `rustStdDepsHash` probe (step 4).

### 7. macOS test slowness
The wasm64 debug test suite runs under `wasmtime` in the nix sandbox. On
macOS/arm64 it can be **~10x slower** than Linux, sometimes appearing to hang.
This is likely due to the system virus scanner (XProtect/MRT) intercepting
wasm memory operations, not a code regression.

**Do not reduce the test object counts** — instead push to CI and let Linux
confirm correctness. Then trigger the `nightly-macos-test` GitHub Actions
workflow on the branch to populate cachix with macOS binaries:
```sh
gh workflow run nightly-macos-test --repo caffeinelabs/motoko --ref <branch>
```
Once cachix is populated, local `nix build` downloads pre-built artifacts
instead of running the slow tests.

**`nightly-macos-test` has two distinct macOS failure modes:**

1. **OOM on `systems-go-tests`** — fails in ~4 min with `patch: **** out of memory`
   while building `ocaml4.14.2-merlin-*` from scratch (before any tests run).
   Runner memory pressure, not a code issue. Re-trigger the workflow; each run
   pushes more artifacts to cachix, and eventually merlin gets cached too.

2. **Slow `gc-tests` / `common-tests`** — same XProtect/wasm64 malaise as local:
   these jobs run but take 10× longer than Linux. They will eventually finish and
   push their artifacts to cachix. Be patient — do not cancel them.

**Re-trigger pattern:** keep re-triggering until all jobs pass; each run makes
progress by caching more derivations:
```sh
gh workflow run nightly-macos-test --repo caffeinelabs/motoko --ref <branch>
```

### 8. Commit
```
chore: bump rustc-nightly to YYYY-MM-DD

- nightly date: old → new
- rust-overlay: updated to YYYY-MM-DD
- rustStdDepsHash: updated for new nightly
- rts/Makefile: <list any Makefile fixes>
- wasm{32,64}-none-shared.json: <if changed>
- motoko-rts: bump foo =X.Y.Z → =X.Y+1.Z
- cargo update: motoko-rts, motoko-rts-tests
```

### 9. Push and watch CI
```sh
git push --force-with-lease origin gabor/bump-nightly-rustc
gh run watch <run-id> --repo caffeinelabs/motoko
```

## Makefile Structure (as of 2026-04-01)

```makefile
RTS_COMPILE_FLAGS=-C target-feature=+bulk-memory
RTS_RELEASE_COMPILE_FLAGS=$(RTS_COMPILE_FLAGS) -Zunstable-options -Cpanic=immediate-abort
NIGHTLY_CARGO_OPTIONS=-Zjson-target-spec -Zbuild-std=core,alloc
NIGHTLY_RELEASE_CARGO_OPTIONS=$(NIGHTLY_CARGO_OPTIONS) --release -Zbuild-std-features="optimize_for_size"
```

## Notes
- `rustStdDepsHash` has been stable (`sha256-ZMCepUZNyqXZcR3EduSV38zFbI89WneU1iTXj3L38RA=`)
  from at least 2026-02-15 through 2026-03-31.
- `libm` is exact-pinned (`=0.2.x`) in `motoko-rts/Cargo.toml` — bump it if
  `cargo update` in `motoko-rts-tests` moves it and the vendor fails.
- The nix sandbox uses `pkgs.wasmtime` from the pinned nixpkgs rev — check
  `wasmtime` version doesn't regress performance between nixpkgs bumps.
