#!/usr/bin/env bash
# unit_fuel: instructions per maintenance unit, by the cost meter. The nt-cost module's `units_*` exports
# build the tables of 56 scenarios (their names in crates/nt-cost-runner/src/bin/units.rs: mixed rows, 64
# KiB-class values, hundreds of leaves, long shared prefixes, schema versions, writes and clears between units,
# running records, Bool and tag leaves, an index added, and the adversarial ones of crates/nt-cost/src/stress.rs)
# and run their seals, compactions, column releases, drops, index builds and the drain one unit per call; the
# runner meters every call under wasmtime 42.0.1 (memory64 on) by the replica's own cost model (each
# instruction's cost as the IC's instrumentation counts it for a 64-bit memory, bulk memory instructions by
# their lengths, each meter reading by its system call's fee), once with the units cut by their own
# instruction estimates and once with that count as their host meter, counts each unit's changed 4 KiB pages,
# and fails when a unit passes 96,000 instructions, 17 changed pages or 164,000 charged, or, with the meter,
# when the work any maintenance decision lets begin costs more than the look-ahead the decision took.
#
# Usage: scripts/unit_fuel.sh [out file]   (default: bench/results/unit_fuel/results.md)
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25
OUT=${1:-bench/results/unit_fuel/results.md}

echo "== the cost module with the meter, and again with the crate's decision notes, wasm64 ($NIGHTLY, -Z build-std)"
CARGO_TARGET_DIR=target/fuel cargo +$NIGHTLY build -q -p nt-cost --lib --release --target wasm64-unknown-unknown \
    -Z build-std=std,panic_abort --features fuel-meter
CARGO_TARGET_DIR=target/fuel-notes cargo +$NIGHTLY build -q -p nt-cost --lib --release --target wasm64-unknown-unknown \
    -Z build-std=std,panic_abort --features meter-notes
MODULE=target/fuel/wasm64-unknown-unknown/release/nt_cost.wasm
NOTES=target/fuel-notes/wasm64-unknown-unknown/release/nt_cost.wasm

echo "== the runner (its own workspace and toolchain: wasmtime 42.0.1 needs Rust 1.91 or later)"
(cd crates/nt-cost-runner && cargo build -q --release --locked --bin units)

mkdir -p "$(dirname "$OUT")"
crates/nt-cost-runner/target/release/units "$MODULE" "$OUT" "$NOTES"
echo "results in $OUT"
