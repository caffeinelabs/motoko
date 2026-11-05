//! The workloads keep their trees on the stack: the canister's stack is made large enough for them.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("wasm") {
        println!("cargo:rustc-link-arg-cdylib=-zstack-size=16777216");
    }
}
