#!/usr/bin/env bash
# cost_vs_ref: wasmtime fuel per typed-map operation, and the gate on it. One wasm64 module runs each
# operation three ways: a transcription of today's PBTreeMap, the crate's typed tree with CHECKED false and
# with CHECKED true (recorded for information). The runner meters every call and writes results.json and
# results.md.
#
# The gate, on the CHECKED-false tree: get and insert with room cost at or below what today's map costs (the
# mean at most 0.05 fuel above, no single sample more than 1 above); cheaper is a gain; every other row costs at most max(4 fuel, 0.5% of today's) more on average. A row above fails the
# script. Rows below are recorded with their negative difference.
#
# The harness is pinned: wasmtime 42.0.1 (the runner's locked dependency), memory64 and fuel consumption on
# with every other setting default, 200 samples per row, and the module built by nightly-2026-07-25 with
# -Z build-std in one codegen unit, so that inlining decisions do not depend on how code is split across
# units.
#
# Usage: scripts/cost_vs_ref.sh [out dir]   (default: bench/results/cost_vs_ref)
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25
WASMTIME=42.0.1
SAMPLES=200
OUT=${1:-bench/results/cost_vs_ref}

grep -q -A1 '^name = "wasmtime"$' crates/nt-cost-runner/Cargo.lock \
    && grep -A1 '^name = "wasmtime"$' crates/nt-cost-runner/Cargo.lock | grep -q "version = \"$WASMTIME\"" \
    || { echo "the runner must lock wasmtime $WASMTIME"; exit 1; }

echo "== the cost module, wasm64 ($NIGHTLY, -Z build-std, one codegen unit)"
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    cargo +$NIGHTLY build -q -p nt-cost --lib --release --target wasm64-unknown-unknown -Z build-std=std,panic_abort
MODULE=target/wasm64-unknown-unknown/release/nt_cost.wasm

echo "== the runner (its own workspace and toolchain: wasmtime $WASMTIME needs Rust 1.91 or later)"
(cd crates/nt-cost-runner && cargo build -q --release --locked)
RUNNER=crates/nt-cost-runner/target/release/nt-cost-runner

rev=$(git rev-parse --short HEAD)
git diff --quiet HEAD -- crates || rev="$rev, uncommitted changes"
BUILD="nt-cost for wasm64-unknown-unknown, $(rustc +$NIGHTLY --version), -Z build-std=std,panic_abort, release profile (opt-level 3, panic abort, codegen-units 1); source $rev"

echo "== metering, $SAMPLES samples per row"
NT_COST_BUILD="$BUILD" "$RUNNER" "$MODULE" "$OUT" "$SAMPLES"
echo "results in $OUT; every row within the gate"
