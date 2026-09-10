The Motoko RTS (C and Rust parts)
=================================

This directory contains the parts of the Motoko runtime implemented in Rust.

tl;dr
-----

If you just want to get RTS wasm files in this directory, make sure you're in
the Nix shell (either using `nix develop` or using `direnv`) then run:

    make -C rts

from the top-level directory of the Motoko repository.

Compilation
-----------

Running `make` should produce RTS Wasm files (different versions).

If run within the Nix shell, the environment variables `WASM_CLANG` and `WASM_LD`
should point to suitable binaries (we track a specific unreleased version of
`llvm`). If not present, the `Makefile` will try to use `clang-21` and
`wasm-ld-21`.

The runtime compiles and links in [libtommath]. It needs the source, so
`nix build` and `nix develop` will set the environment variable `TOMMATHSRC` to
point to the source in `/nix/store`.

If not present, the `Makefile` will look in `../../libtommath`, i.e. parallel
to the `motoko` repository; this is useful if you need to hack on libtommath.

[libtommath]: https://github.com/libtom/libtommath

Exporting and importing functions
---------------------------------

Import and export as if you are importing from or exporting to a C library. Examples:

```rust
// Expects bigint_trap to be provided at link time. The function should follow
// C calling conventions
extern "C" {
    fn bigint_trap() -> !;
}

// Provides bigint_add function. The function follows C calling conventions
#[no_mangle]
extern "C" fn bigint_add(...) { ... }
```

libtommath and memory management
--------------------------------

We have to make libtommath’s memory management (which expects functions
`alloc`, `calloc` and `realloc`) work with the Motoko runtime.
See `motoko-rts/src/bigint.rs` for the technical details.

Rust build
----------

To build Motoko RTS in nix we need pre-fetch Rust dependencies. This works in
`nix build` by:

 * Building a directory with vendored sources in `nix/rts.nix`

 * Configuring `cargo` to use that vendored directory (see `preBuild`)

If you change dependencies (e.g. bump versions, add more crates), Make sure that
`motoko-rts-tests/Cargo.lock` is up to date. This can be done by running
`cargo build --target=wasm64-unknown-unknown --features enhanced_orthogonal_persistence` in `motoko-rts-tests/` directory (see the `test64` target in `rts/Makefile`).

**Updating rustc**: see [`.agents/skills/bump-rust-nightly/SKILL.md`](../.agents/skills/bump-rust-nightly/SKILL.md) for the full recipe — nightly date, `rustStdDepsHash` probe, Cargo lockfile updates, common compiler-error fixes, and CI-trigger pattern.

Running RTS tests
-----------------

- Build tests using the wasm64 EOP target: `make test` (in `rts/`, builds the
  `test64` variant of `motoko-rts-tests` and runs every module under wasmtime)
- Or manually: `cargo build --target=wasm64-unknown-unknown --features enhanced_orthogonal_persistence`
  in `motoko-rts-tests/`, then run with
  `wasmtime -W memory64 --invoke test_<module> target/wasm64-unknown-unknown/debug/motoko-rts-tests.wasm`

Debugging the RTS
-----------------

The RTS and its test suite build exclusively for the 64-bit wasm64 target
(`motoko-rts-tests/build.rs` accepts only `wasm64-unknown-unknown`). The
i686 native debug recipe that used to live here was removed together with the
32-bit (classical) build; debug RTS code by running it under wasmtime as
described under *Running RTS tests* above.
