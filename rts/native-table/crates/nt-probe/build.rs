//! On wasm, the probe's count export `nt_count_bar` keeps its table memory (`COUNT_MEM`, 100 MiB) on the
//! stack: the module's stack is made larger than that.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("wasm") {
        println!("cargo:rustc-link-arg-cdylib=-zstack-size=167772160");
    }
}
