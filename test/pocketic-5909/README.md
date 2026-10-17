# Issue 5909: standalone PocketIC runner

Requires Node.js >=22.12, npm, Git, and curl. No Nix, Rust, or OCaml build.
From this directory:

```sh
npm ci
npm test
```

If npm reports that the PocketIC postinstall script was blocked, explicitly
approve that package and rerun its install script:

```sh
npm install-scripts approve @dfinity/pic
npm rebuild @dfinity/pic
npm test
```

This script downloads the platform-specific PocketIC binary. Approval is
scoped to that dependency; there is no need to enable all install scripts.

The dependency install downloads PocketIC for macOS/Linux and the current
architecture. The runner downloads and verifies the official moc 2.0.0 JS
compiler, substitutes exactly its embedded `@new_async` implementation with
the checkout's implementation, and compiles both variants of the same actor.
It executes the commands from `test/run-drun/await-trapped.drun` on isolated
PocketIC instances. The baseline must fail the four assertion queries; the
patched version must pass. Success/rejection and unawaited-call controls must
pass on both. A failed assertion in the baseline is expected, not a failed
test run. The process exits unsuccessfully for any unexpected outcome.

To use an existing binary instead of downloading PocketIC during install:

```sh
npm ci --ignore-scripts
POCKET_IC_BIN=/absolute/path/to/pocket-ic npm test
```

The client is pinned to @dfinity/pic 0.25.0, whose default PocketIC is 16.0.0.
An older existing binary may not be compatible with this client's HTTP API.
To reuse a running compatible server, set `POCKET_IC_URL=http://127.0.0.1:PORT`.
The runner only creates and deletes its own instances; it does not stop an
externally supplied server.

`npm test -- --compile-only` verifies both compilations without starting a
server. Downloads and generated Wasm live in the ignored `.cache/` directory.

This focused check does not replace CI's native compiler build, sanity checks,
or other regressions, and does not regenerate the `.drun.ok` fixture. The JS
compiler uses the released 2.0.0 compiler and RTS with only the future Prelude
implementation replaced. CI builds the whole checkout instead.

Validation at preparation time: both variants compile without diagnostics;
PocketIC execution remains unverified because the preparation environment
prohibits Unix sockets. Run `npm test` to perform the actual IC check.
