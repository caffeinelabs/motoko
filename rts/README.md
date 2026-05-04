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
`llvm`). If not present, the `Makefile` will try to use `clang-19` and
`wasm-ld-19`.

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
`cargo build --target=wasm32-wasip1` in `motoko-rts-tests/` directory.

**Updating rustc**: see [`.agents/skills/bump-rust-nightly/SKILL.md`](../.agents/skills/bump-rust-nightly/SKILL.md) for the full recipe — nightly date, `rustStdDepsHash` probe, Cargo lockfile updates, common compiler-error fixes, and CI-trigger pattern.

Running RTS tests
-----------------

- Build tests using rustc WASI target: `cargo build --target=wasm32-wasip1`
- Run with wasmtime: `wasmtime target/wasm32-wasip1/debug/motoko-rts-tests.wasm`

Debugging the RTS
-----------------

It is possible to build the RTS and test suite for i686 and debug using native
debug tools like gdb and rr. You first need to build tommath-related files.
This is easiest to do in the Nix shell:

- (in `rts/`) `make _build/libtommath_i686.a` (this step requires headers and
  libraries for the target)
- (in `rts/`) `make _build/tommath_bindings.rs` (this step requires `bindgen`)

After these you can build the test suite for `i686-unknown-linux-gnu` target
outside of `nix-shell`. If you don't have the target installed already, install with

- `rustup +nightly target install i686-unknown-linux-gnu`

Now build i686 executable:

- (in `rts/motoko-rts-tests`) `cargo +nightly build --target=i686-unknown-linux-gnu`

Now you should see an i686 executable
`rts/motoko-rts-tests/target/i686-unknown-linux-gnu/debug/motoko-rts-tests`
that you can debug with e.g. `gdb`.

Ideally all of these steps would be done in `nix develop` or outside, but the
last command does not work in `nix develop` because of missing i686 libraries and
I couldn't figure out how to install those in nix.
