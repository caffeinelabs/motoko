fn main() {
    let target = std::env::var("TARGET").unwrap();

    match target.as_str() {
        "wasm64-unknown-unknown" => {
            println!("cargo:rustc-link-search=native=../_build");
            println!("cargo:rustc-link-lib=static=tommath_wasm64");
        }

        "wasm32-wasip1" => {
            println!("cargo:rustc-link-search=native=../_build");
            println!("cargo:rustc-link-lib=static=tommath_wasm32");
        }

        "i686-unknown-linux-gnu" => {
            println!("cargo:rustc-link-search=native=../_build");
            println!("cargo:rustc-link-lib=static=tommath_i686");
        }

        other => panic!("Don't know how to link the runtime system for '{}'", other),
    }
}
