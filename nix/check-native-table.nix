{ pkgs }:
# The native-table crate's own gate (see rts/native-table/scripts/check.sh): its lints and its tests,
# with the RTS's Rust toolchain. The crate's workspace has no dependencies to vendor.
pkgs.stdenv.mkDerivation {
  name = "check-native-table";
  buildInputs = [ pkgs.rust-nightly ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.libiconv ];
  src = ../rts/native-table;
  doCheck = true;
  phases = "unpackPhase checkPhase installPhase";
  checkPhase = ''
    export CARGO_HOME=$PWD/cargo-home
    cargo clippy --offline -p native-table --all-targets -- -D warnings
    cargo test --offline --workspace --profile test-fast
  '';
  installPhase = "touch $out";
}
