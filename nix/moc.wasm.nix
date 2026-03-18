{ pkgs, commonBuildInputs }:
pkgs.stdenv.mkDerivation {
  name = "stable-check.wasm";
  src = ../src;
  buildInputs = commonBuildInputs pkgs ++ [
    pkgs.ocamlPackages.wasm_of_ocaml-compiler
    pkgs.binaryen
    pkgs.nodejs
  ];
  buildPhase = ''
    patchShebangs .
    make moc.wasm
  '';
  installPhase = ''
    mkdir -p $out/bin
    cp --verbose --dereference moc.wasm $out/bin/stable-check.wasm.js
    cp -r --dereference _build/default/js/stable_check_wasm.bc.wasm.assets $out/bin/
  '';
  dontStrip = true;
  doInstallCheck = true;
  installCheckPhase = ''
    echo "actor {}" > /tmp/pre.most
    echo "actor {}" > /tmp/post.most
    node $out/bin/stable-check.wasm.js /tmp/pre.most /tmp/post.most
  '';
}
