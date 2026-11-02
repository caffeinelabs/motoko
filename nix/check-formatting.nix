{ pkgs }:
pkgs.stdenv.mkDerivation {
  name = "check-formatting";
  buildInputs = [ pkgs.ocamlformat ];
  src = ../src;
  doCheck = true;
  phases = "unpackPhase checkPhase installPhase";
  installPhase = "touch $out";
  checkPhase = ''
    ocamlformat --check docs/*.{ml,mli}
    ocamlformat --check exes/deser.ml
  '';
}
