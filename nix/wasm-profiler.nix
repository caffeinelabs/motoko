pkgs:
let
  wasm-profiler-src = ../wasm-profiler;
in
{
  wasm-profiler-instrument =
    pkgs.rustPlatform.buildRustPackage {
      name = "wasm-profiler-instrument";

      src = wasm-profiler-src;

      cargoLock = {
        lockFile = ../wasm-profiler/Cargo.lock;
      };

    };

  wasm-profiler-postproc = pkgs.stdenv.mkDerivation rec {
    name = "wasm-profiler-postproc";

    src = wasm-profiler-src;

    buildInputs = [ pkgs.perl ];

    installPhase = ''
      mkdir -p $out/bin
      cp $src/wasm-profiler-postproc.pl $out/bin/wasm-profiler-postproc
    '';
  };

  # the FlameGraph package is a bit inconvenient, with stuff like files.pl in
  # the path. Package a smaller, nicer one, with just the flamegraph tool.
  flamegraph-bin = pkgs.runCommand "flamegraph" { } ''
    mkdir -p $out/bin
    cp ${pkgs.flamegraph}/bin/flamegraph.pl $out/bin/flamegraph
  '';
}
