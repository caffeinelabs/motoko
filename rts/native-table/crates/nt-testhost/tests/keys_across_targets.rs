//! Identical key bytes on every target. The native digest of the probe's key generator must equal the
//! digest the wasm64 probe computes under wasmtime, with NaN canonicalization off and on. Set
//! `NT_PROBE_WASM` to the built probe (scripts/check.sh does) to run the wasm side.

use std::process::Command;

#[test]
fn key_bytes_are_identical_on_every_target() {
    let native = nt_probe::key_digest(12_345, 3_000) as i64;
    let Ok(wasm) = std::env::var("NT_PROBE_WASM") else {
        eprintln!("NT_PROBE_WASM not set: the wasm side is checked by scripts/check.sh");
        return;
    };
    for nan in ["n", "y"] {
        let out = Command::new("wasmtime")
            .args(["run", "-W", &format!("memory64=y,nan-canonicalization={nan}"), "--invoke", "nt_key_digest"])
            .arg(&wasm)
            .args(["12345", "3000"])
            .output()
            .expect("wasmtime runs");
        let got: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().expect("a number");
        assert_eq!(got, native, "nan-canonicalization={nan}");
    }
}
