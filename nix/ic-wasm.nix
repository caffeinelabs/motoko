pkgs: with pkgs.llvmPackages_21; pkgs.rustPlatform.buildRustPackage rec {
  pname = "ic-wasm";
  version = builtins.substring 0 7 src.rev;
  src = pkgs.sources.ic-wasm-src;
  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };
  doCheck = false;

  # use the unwrapped clang with appropriate include paths
  CXX_aarch64-apple-darwin = "${clang-unwrapped}/bin/clang++";
  CXXFLAGS_aarch64-apple-darwin = ''
    -isystem ${libcxx.dev}/include/c++/v1
    -isystem ${clang}/resource-root/include
  '';
  CXX_x86_64-apple-darwin = CXX_aarch64-apple-darwin;
  CXXFLAGS_x86_64-apple-darwin = CXXFLAGS_aarch64-apple-darwin;

  # The C++ files in `wasm-opt-{sys,cxx-sys}` are compiled with libc++ 21
  # headers (see `CXXFLAGS_*-apple-darwin` above), which reference symbols
  # like `std::__1::__hash_memory` that the macOS SDK's libc++ does not
  # provide. Without this, the final link picks `-lc++` from the SDK and
  # fails with "Undefined symbols for architecture arm64". Force the
  # linker to resolve `-lc++` against `llvmPackages_21.libcxx`.
  NIX_LDFLAGS = pkgs.lib.optionalString pkgs.stdenv.isDarwin "-L${libcxx}/lib";
}
