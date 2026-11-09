#!/usr/bin/env bash
# The build gate: every target builds, the lints pass, the tests pass natively, and the probe's
# self-test passes under wasmtime with memory64.
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25

# Rust EOP is a Rust library for upgrades that keep the heap in place.
echo "== the probe module for wasm64-unknown-unknown, the target of Rust EOP canisters"
cargo +$NIGHTLY build -q -p nt-probe --lib --release --target wasm64-unknown-unknown -Z build-std=core,panic_abort
PROBE=target/wasm64-unknown-unknown/release/nt_probe.wasm

echo "== native build, lints and tests (Rust 1.83.0), with the wasm64 probe for cross-target checks"
cargo build -q --workspace
cargo clippy -q -p native-table -p nt-builder --all-targets -- -D warnings
NT_PROBE_WASM="$PWD/$PROBE" cargo test -q --workspace --profile test-fast

echo "== the large suite in release mode: rows_model at 10^6 writes and across schema versions"
NT_LARGE=1 cargo test -q --release -p nt-testhost --test updates_and_rewrites -- rows_model

echo "== probe self-test under wasmtime ($(wasmtime --version))"
out=$(wasmtime run -W memory64=y --invoke nt_selftest $PROBE 2>/dev/null)
[ "$out" = "0" ] || { echo "nt_selftest failed: $out"; exit 1; }

echo "== Motoko's wasm64-none-shared target (copied from motoko/rts/motoko-rts)"
cargo +$NIGHTLY build -q -p native-table --release -Zjson-target-spec -Z build-std=core,alloc \
    --target targets/wasm64-none-shared.json

echo "all checks passed"
