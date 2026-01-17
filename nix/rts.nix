{ pkgs, llvmEnv }:
let
  # Build Rust package cargo-vendor-tools
  cargoVendorTools = pkgs.rustPlatform.buildRustPackage rec {
    name = "cargo-vendor-tools";
    src = ../rts/${name};
    cargoLock = {
      lockFile = ../rts/${name}/Cargo.lock;
    };
  };

  # Path to vendor-rust-std-deps, provided by cargo-vendor-tools
  vendorRustStdDeps = "${cargoVendorTools}/bin/vendor-rust-std-deps";

  # SHA256 of Rust std deps
  rustStdDepsHash = "sha256-N6DDytOiVRhTRbClTOMOITov03Az2ZC5YOrezLM4N4k=";

  # Vendor directory for Rust std deps
  rustStdDeps = pkgs.stdenvNoCC.mkDerivation {
    name = "rustc-std-deps";

    nativeBuildInputs = [ pkgs.curl ];

    buildCommand = ''
      mkdir $out
      cd $out
      ${vendorRustStdDeps} ${pkgs.rust-nightly} .
    '';

    outputHash = rustStdDepsHash;
    outputHashAlgo = "sha256";
    outputHashMode = "recursive";
  };

  # Vendor tarball of the RTS
  rtsDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = ../rts/motoko-rts-tests/Cargo.lock;
  };

  # All dependencies needed to build the RTS, including Rust std deps, to
  # allow `cargo -Zbuild-std`. (rust-lang/wg-cargo-std-aware#23)
  allDeps = pkgs.symlinkJoin {
    name = "merged-rust-deps";
    paths = [
      rtsDeps
      rustStdDeps
    ];
  };
in

pkgs.stdenv.mkDerivation {
  name = "moc-rts";

  src = ../rts;

  nativeBuildInputs = [ pkgs.makeWrapper pkgs.removeReferencesTo pkgs.cacert ];

  buildInputs = with pkgs; [
    llvmPackages_19.clang
    llvmPackages_19.bintools
    rust-nightly
    wasmtime
    rust-bindgen
    python3
    wabt
  ] ++ pkgs.lib.optional pkgs.stdenv.isDarwin [
    libiconv
  ];

  preBuild = ''
    export CARGO_HOME=$PWD/cargo-home

    # This replicates logic from nixpkgs’ pkgs/build-support/rust/default.nix
    mkdir -p $CARGO_HOME
    echo "Using vendored sources from ${rtsDeps}"
    unpackFile ${allDeps}
    cat > $CARGO_HOME/config.toml <<__END__
      [source."crates-io"]
      "replace-with" = "vendored-sources"

      [source."vendored-sources"]
      "directory" = "$(stripHash ${allDeps})"
    __END__


    ${llvmEnv}
    export TOMMATHSRC=${pkgs.sources.libtommath-src}
  '';

  doCheck = true;

  checkPhase = ''
    make test
  '';

  installPhase = ''
    mkdir -p $out/rts
    cp mo-rts-non-incremental.wasm $out/rts
    cp mo-rts-non-incremental-debug.wasm $out/rts
    cp mo-rts-incremental.wasm $out/rts
    cp mo-rts-incremental-debug.wasm $out/rts
    cp mo-rts-eop.wasm $out/rts
    cp mo-rts-eop-debug.wasm $out/rts
  '';

  # This needs to be self-contained. Remove mention of nix path in debug
  # message.
  preFixup = ''
    remove-references-to \
      -t ${pkgs.rust-nightly} \
      $out/rts/mo-rts-non-incremental.wasm \
      $out/rts/mo-rts-non-incremental-debug.wasm \
      $out/rts/mo-rts-incremental.wasm \
      $out/rts/mo-rts-incremental-debug.wasm \
      $out/rts/mo-rts-eop.wasm \
      $out/rts/mo-rts-eop-debug.wasm

    for rtsDep in $(find ${rtsDeps} -type l -exec readlink {} +); do
      remove-references-to \
        -t "$rtsDep" \
        $out/rts/mo-rts-non-incremental.wasm \
        $out/rts/mo-rts-non-incremental-debug.wasm \
        $out/rts/mo-rts-incremental.wasm \
        $out/rts/mo-rts-incremental-debug.wasm \
        $out/rts/mo-rts-eop.wasm \
        $out/rts/mo-rts-eop-debug.wasm
    done
  '';

  allowedRequisites = [ ];
}
