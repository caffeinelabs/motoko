{ pkgs }:
pkgs.stdenv.mkDerivation {
  name = "check-rts-formatting";
  buildInputs = [ pkgs.rust-nightly pkgs.rustfmt ];
  src = ../rts;
  doCheck = true;
  phases = "unpackPhase checkPhase installPhase";
  checkPhase = ''
    echo "If this fails, run `make -C rts format`"
    cargo fmt --verbose --manifest-path motoko-rts/Cargo.toml -- --check
    cargo fmt --verbose --manifest-path motoko-rts-tests/Cargo.toml -- --check
    cargo fmt --verbose --manifest-path native-table/Cargo.toml --all -- --check
  '';
  installPhase = "touch $out";
}
