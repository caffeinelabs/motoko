#!/usr/bin/env bash
# The build gate: every target builds, the lints pass, the tests
# pass natively, the panic scan finds no panicking function reachable from any probe export, the
# probe's self-test passes under wasmtime with memory64, the counts stay within OQL's bars, the write
# workloads run on PocketIC, and every maintenance unit stays within its bound by the replica's cost model.
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25

# Rust EOP is a Rust library for upgrades that keep the heap in place.
echo "== the probe module for wasm64-unknown-unknown, the target of Rust EOP canisters"
cargo +$NIGHTLY build -q -p nt-probe --lib --release --target wasm64-unknown-unknown -Z build-std=core,panic_abort
PROBE=target/wasm64-unknown-unknown/release/nt_probe.wasm

echo "== native build, lints and tests (Rust 1.83.0), with the wasm64 probe for cross-target checks"
cargo build -q --workspace
cargo clippy -q -p native-table --all-targets -- -D warnings
NT_PROBE_WASM="$PWD/$PROBE" cargo test -q --workspace --profile test-fast

echo "== the large suite in release mode: rows_model at 10^6 writes and across schema versions, rewrite_units writing while a compaction runs, post-snapshot patches past 254 units"
NT_LARGE=1 cargo test -q --release -p nt-testhost --test updates_and_rewrites -- rewrite_units rows_model \
    post_snapshot_units_over_255

# The crate's side of the golden typed nodes runs in the tests above (nt-testhost, test golden_nodes). Their
# reference side, the crate nt-eopref, builds rust-eop, the repository of Rust EOP, from a local clone; a tree
# without that crate skips it.
if [ -f crates/nt-eopref/Cargo.toml ]; then
    echo "== golden typed nodes, reference side: today's PBTreeMap (rust-eop's map.rs, host mode) equals golden/nodes"
    (cd crates/nt-eopref && cargo run -q --release)
else
    echo "== golden typed nodes, reference side: skipped, the crate nt-eopref is not in this tree"
fi

echo "== panic scan"
cargo run -q -p nt-panicscan -- $PROBE --seed nt_

echo "== probe self-test under wasmtime ($(wasmtime --version))"
out=$(wasmtime run -W memory64=y --invoke nt_selftest $PROBE 2>/dev/null)
[ "$out" = "0" ] || { echo "nt_selftest failed: $out"; exit 1; }

echo "== the cost meter's calibration canister (wasm64; its own lock file)"
cargo +$NIGHTLY build -q --release --target wasm64-unknown-unknown -Z build-std=core,panic_abort \
    --manifest-path crates/nt-calib/Cargo.toml
CALIB=crates/nt-calib/target/wasm64-unknown-unknown/release/nt_calib.wasm

echo "== count costs against OQL's bars (the cost meter at its recorded calibration; its own toolchain and lock file)"
cargo +1.97.0 run -q --release --manifest-path crates/nt-fuel/Cargo.toml -- $CALIB

# PocketIC 14.0.0, the replica's local test instance the costs are measured on; NT_POCKET_IC names another
# copy of its server binary.
PIC="${NT_POCKET_IC:-$HOME/Library/Caches/mops/pocket-ic/14.0.0/pocket-ic}"
echo "== write costs on PocketIC: index inserts, deletes and counts, a table's writes, index builds each unit within 96,000 instructions (nt-fuel writes)"
[ -x "$PIC" ] || { echo "no PocketIC server binary at $PIC (set NT_POCKET_IC)"; exit 1; }
WRITES=target/nt-fuel-writes.md
cargo +1.97.0 run -q --release --manifest-path crates/nt-fuel/Cargo.toml -- writes $CALIB "$PIC" > $WRITES \
    || { cat $WRITES; echo "a write workload failed or a build unit passed its bound ($WRITES)"; exit 1; }

echo "== maintenance units by the replica's cost model: each within 96,000 instructions and 17 changed blocks, every decision's look-ahead held"
# Their own target directories: the module links std, the probe above only core; the second is built with the
# crate's decision notes, which the runner checks.
CARGO_TARGET_DIR=target/fuel cargo +$NIGHTLY build -q -p nt-cost --lib --release --target wasm64-unknown-unknown \
    -Z build-std=std,panic_abort --features fuel-meter
CARGO_TARGET_DIR=target/fuel-notes cargo +$NIGHTLY build -q -p nt-cost --lib --release --target wasm64-unknown-unknown \
    -Z build-std=std,panic_abort --features meter-notes
UNITS_WASM="$PWD/target/fuel/wasm64-unknown-unknown/release/nt_cost.wasm"
NOTES_WASM="$PWD/target/fuel-notes/wasm64-unknown-unknown/release/nt_cost.wasm"
(cd crates/nt-cost-runner && NT_COST_WASM="$UNITS_WASM" NT_COST_NOTES_WASM="$NOTES_WASM" \
    cargo test -q --release --locked --bin units)

echo "== Motoko's wasm64-none-shared target (copied from motoko/rts/motoko-rts)"
cargo +$NIGHTLY build -q -p native-table --release -Zjson-target-spec -Z build-std=core,alloc \
    --target targets/wasm64-none-shared.json

echo "all checks passed"
