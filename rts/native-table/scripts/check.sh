#!/usr/bin/env bash
# The build gate: the crate builds natively and for Motoko's target, the lints pass, and the tests
# pass.
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25

echo "== native build, lints and tests (Rust 1.83.0)"
cargo build -q --workspace
cargo clippy -q -p native-table --all-targets -- -D warnings
cargo test -q --workspace --profile test-fast

echo "== the large suite in release mode: the model test, 4 tables of 10^5 writes"
NT_MODEL_OPS=100000 NT_MODEL_TABLES=4 cargo test -q --release -p nt-testhost --test model -- model --exact

echo "== Motoko's wasm64-none-shared target (copied from motoko/rts/motoko-rts)"
cargo +$NIGHTLY build -q -p native-table --release -Zjson-target-spec -Z build-std=core,alloc \
    --target targets/wasm64-none-shared.json

echo "all checks passed"
