# QA tool `qc-motoko`

This tool generates Motoko AST fragments and executes them,
checking for the same outcome as the Haskell (here: reference)
implementation.

In order to compile and test it expects following tools to be in the
`PATH`:
- `moc` (the Motoko compiler)
- `wasmtime`

## How to spot failures

`qc-motoko` is run from the CI infrastructure on each commit. On failures, something like

```
  Properties
    (checked by QuickCheck)
      expected failures:  FAIL (0.08s)
        *** Failed! Assertion failed (after 1 test):
        Failing "let _ = (-(nat32ToNat(((((0) : Nat32) * ((65535) : Nat32)) % (((65535) : Nat32) * ((0) : Nat32))))));"
        Use --quickcheck-replay=232458 to reproduce.
      expected successes: OK (13.22s)
        +++ OK, passed 100 tests.

1 out of 2 tests failed (13.30s)
```

will appear in the CI log. The relevant recipe for reliable reproduction is the
`--quickcheck-replay=232458` hint.

## How to run locally

With this piece of information you can run the identical test locally:
set `replay = 232458;` in `nix/tests.nix`, then run from your `motoko`
directory:
``` shell
$ nix build -L .#tests.qc
```

## Running in `moc` and `wasmtime`

Of course you can dump the failing test case into a file, compile it
to WASM, and execute it in `wasmtime`:

``` shell
$ moc -wasi-system-api snippet.mo
$ wasmtime -W memory64 -W multi-memory -W bulk-memory snippet.wasm
```

In tests under category *expected failures*, this should trap.
For *expected successes* it should execute without errors.

## Troubleshooting

When something doesn't build like expected, check for a file called (approximately)
`test/random/.ghc.environment.x86_64-darwin-8.6.4`. This usually trips
up `nix-build`'s ability to build `qc-motoko` locally, and should
be deleted.

## 0 -- 2019-08-08

* First version. Released on a suspecting world. See also [GitHub issue 609](https://github.com/dfinity/motoko/pull/609).
