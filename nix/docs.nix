{ pkgs, js, base-src, core-src }:
# The pandoc/Docusaurus build is replaced by a Starlight site (doc/site/).
# Rendered docs are published at docs.internetcomputer.org/languages/motoko/.
# This derivation copies the Markdown source so it remains available as a
# Nix output and keeps docs.buildInputs accessible for the dev shell.
pkgs.stdenv.mkDerivation {
  name = "docs";
  src = ../doc/md;

  buildPhase = "true";

  installPhase = ''
    mkdir -p $out/md
    cp -r . $out/md/
  '';
}
