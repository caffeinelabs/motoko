fn main() {
    // Pass git hash as compile-time env for test seed derivation
    let git_hash = std::env::var("RTS_TEST_GIT_HASH").unwrap_or_else(|_| "4711".to_string());
    println!("cargo:rustc-env=RTS_TEST_GIT_HASH={}", git_hash);

    let target = std::env::var("TARGET").unwrap();

    if target == "wasm64-unknown-unknown" {
        println!("cargo:rustc-link-search=native=../_build");
        println!("cargo:rustc-link-lib=static=tommath_wasm64");
    } else {
        panic!(
            "Don't know how to link the runtime system for '{}' (only wasm64 is supported)",
            target
        );
    }
}
