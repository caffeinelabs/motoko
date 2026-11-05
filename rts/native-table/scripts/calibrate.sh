#!/usr/bin/env bash
# The cost meter's calibration on PocketIC, the replica's local test instance. The tree mix and a copy-heavy
# unit run once under the meter (wasmtime fuel aware of bulk memory instructions) and once on the replica;
# the run prints the fitted ratio r and its band, to be recorded as `R` and `BAND` in crates/nt-fuel/src/main.rs,
# and every count row of the bars as the replica measures it beside its estimate and its bar.
#
# Usage: scripts/calibrate.sh <pocket-ic server binary, version 14.0.0>
set -euo pipefail
cd "$(dirname "$0")/.."
NIGHTLY=nightly-2026-07-25
SERVER=${1:?usage: scripts/calibrate.sh <pocket-ic server binary, version 14.0.0>}

cargo +$NIGHTLY build -q --release --target wasm64-unknown-unknown -Z build-std=core,panic_abort \
    --manifest-path crates/nt-calib/Cargo.toml
cargo +1.97.0 run -q --release --manifest-path crates/nt-fuel/Cargo.toml -- calibrate \
    crates/nt-calib/target/wasm64-unknown-unknown/release/nt_calib.wasm "$SERVER"
