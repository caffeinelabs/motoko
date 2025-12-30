pkgs: with pkgs.llvmPackages_19; pkgs.rustPlatform-stable.buildRustPackage rec {
  pname = "ic-wasm";
  version = builtins.substring 0 7 src.rev;
  src = pkgs.sources.ic-wasm-src;
  cargoLock = {
    lockFile = "${src}/Cargo.lock";
    outputHashes = {
         "wasm-opt-0.121.0" = "sha256-eQ3FD/BFO8x8GT38t4BVCiD+vvnAsDPtU6tMRZyNEu0=";
    };
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
}
