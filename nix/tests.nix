{ pkgs, llvmEnv, esm, commonBuildInputs, debugMoPackages, test-runner, core-src }:
with debugMoPackages;
let
  # The following were previously arguments to default.nix but flakes don't accept options yet.
  # This will be the case when we get Configurable Flakes:
  # https://www.youtube.com/live/yhfDtRRTmY8?si=0Mn-YBTMqcUl9Hb5&t=17974
  accept-bench = "x86_64-linux";
  replay = 0;

  testDerivationArgs = {
    # by default, an empty source directory. how to best get an empty directory?
    src = builtins.path { name = "empty"; path = ./.; filter = p: t: false; };
    phases = "unpackPhase checkPhase installPhase";
    doCheck = true;
    installPhase = "touch $out";
  };

  testDerivationDeps =
    (with pkgs; [ wabt bash perl getconf moreutils nodejs_20 ]) ++
    [ filecheck pkgs.wasmtime ];

  filecheck = pkgs.runCommandNoCC "FileCheck" { } ''
    mkdir -p $out/bin
    cp ${pkgs.llvm}/bin/FileCheck $out/bin
  '';

  # extra deps for test/ld
  ldTestDeps =
    with pkgs.llvmPackages_19; [ lld clang ];

  testDerivation = args:
    pkgs.stdenv.mkDerivation (testDerivationArgs // args);

  # we test each subdirectory of test/ in its own derivation with
  # cleaner dependencies, for more parallelism, more caching
  # and better feedback about what aspect broke
  # so include from test/ only the common files, plus everything in test/${dir}/
  test_src = dir:
    with pkgs.lib;
    cleanSourceWith {
      filter = path: type:
        let relPath = removePrefix (toString ../test + "/") (toString path); in
        type != "directory"
        || hasPrefix "${dir}/" "${relPath}/"
        || hasPrefix "core-stub/" "${relPath}/"
        || hasPrefix "base-stub/" "${relPath}/";
      src = ../test;
      name = "test-${dir}-src";
    };

  acceptable_subdir = accept: dir: deps:
    testDerivation ({
      src = test_src dir;
      buildInputs = deps ++ testDerivationDeps;

      checkPhase = ''
        patchShebangs .
        ${llvmEnv}
        export ESM=${esm}
        export MOTOKO_CORE="${core-src}"
        type -p moc && moc --version
        ${if dir == "run-drun" 
          then "make -C ${dir}${pkgs.lib.optionalString (pkgs.system != "x86_64-darwin") " parallel -j4"} ${pkgs.lib.optionalString accept " accept"}"
          else "make -C ${dir}${pkgs.lib.optionalString accept " accept"}"
        }
      '';
    } // pkgs.lib.optionalAttrs accept {
      installPhase = pkgs.lib.optionalString accept ''
        export MOTOKO_CORE="${core-src}"
        mkdir -p $out/share
        cp -v ${dir}/ok/*.ok $out/share
      '';
    } // pkgs.lib.optionalAttrs (builtins.elem test-runner deps) {
      POCKET_IC_BIN = "${pkgs.pocket-ic.server}/bin/pocket-ic-server";
      SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
    });

  test_subdir = dir: deps: acceptable_subdir false dir deps;

  # Run a variant with sanity checking on
  snty_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--sanity-checks";
    };

  snty_compacting_gc_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--sanity-checks --compacting-gc";
    };

  snty_generational_gc_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--sanity-checks --generational-gc";
    };

  snty_incremental_gc_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--sanity-checks --incremental-gc";
    };

  enhanced_orthogonal_persistence_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--enhanced-orthogonal-persistence";
    };

  snty_enhanced_orthogonal_persistence_subdir = dir: deps:
    (test_subdir dir deps).overrideAttrs {
      EXTRA_MOC_ARGS = "--sanity-checks --enhanced-orthogonal-persistence";
    };

  perf_subdir = accept: dir: deps:
    (acceptable_subdir accept dir deps).overrideAttrs (args: {
      checkPhase = ''
        mkdir -p $out
        export PERF_OUT=$out/stats.csv
      '' + args.checkPhase + ''
        # export stats to hydra
        mkdir -p $out/nix-support
        tr '/;' '_\t' < $out/stats.csv > $out/nix-support/hydra-metrics

        # sanity check
        if ! grep -q ^gas/ $out/stats.csv
        then
          echo "perf stats do not include gas. change in pocket-ic output format?" >&2
          exit 1
        fi
      '';
    });

  haskellPackages = pkgs.haskellPackages.override {
    overrides = _hself: hsuper: {
      qc-motoko = hsuper.callCabal2nix "qc-motoko" ../test/random { };
    };
  };

  qc = testDerivation {
    buildInputs =
      [ moc pkgs.wasmtime haskellPackages.qc-motoko ];
    checkPhase = ''
      export LANG=C.utf8 # for haskell
      qc-motoko${pkgs.lib.optionalString (replay != 0)
          " --quickcheck-replay=${toString replay}"}
    '';
  };

  unit = testDerivation {
    # The rule for src/pipeline/dune will attempt to copy some files from the
    # test directory to be run by src/pipeline/test_field_srcs.ml. We create src
    # and test (the latter only with the wanted subdirectories) so that the dune
    # rule will be able to copy.
    src = pkgs.runCommand "project-sources" { } ''
      mkdir -p $out/src $out/test
      cp -r ${../src}/* $out/src
      cp -r ${../test}/{run,run-drun,perf,bench} $out/test
    '';
    buildInputs = commonBuildInputs pkgs;
    checkPhase = ''
      cd src
      patchShebangs .
      make DUNE_OPTS="--display=short" unit-tests
    '';
    installPhase = ''
      touch $out
    '';
  };

  candid = testDerivation {
    buildInputs = [ moc pkgs.wasmtime candid-tests ];
    checkPhase = ''
      candid-tests -i ${pkgs.sources.candid-src}/test
    '';
  };

  # wasm-profiler is not compatible with passive data segments and memory64
  # profiling-graphs = testDerivation {
  #  src = test_src "perf";
  #  buildInputs =
  #    (with pkgs; [ perl wabt wasm-profiler-instrument wasm-profiler-postproc flamegraph-bin ]) ++
  #    [ moc pkgs.drun ];
  #  checkPhase = ''
  #    patchShebangs .
  #    type -p moc && moc --version
  #    type -p drun && drun --help
  #    ./profile-report.sh
  #  '';
  #  installPhase = ''
  #    mv _profile $out;
  #    mkdir -p $out/nix-support
  #    echo "report flamegraphs $out index.html" >> $out/nix-support/hydra-build-products
  #  '';
  #};
  fix_names = builtins.mapAttrs (name: deriv:
    deriv.overrideAttrs { name = "test-${name}"; }
  );

  coverage = testDerivation {
    # this runs all subdirectories, so let's just depend on all of test/
    src = ../test;
    buildInputs =
      builtins.attrValues coverage_bins ++
      [ pkgs.ocamlPackages.bisect_ppx ] ++
      testDerivationDeps ++
      ldTestDeps;

    checkPhase = ''
      patchShebangs .
      ${llvmEnv}
      export ESM=${esm}
      export MOTOKO_CORE="${core-src}"
      export SOURCE_PATHS="${
        builtins.concatStringsSep " " (map (d: "${d}/src") (builtins.attrValues coverage_bins))
      }"
      type -p moc && moc --version
      make coverage
    '';
    installPhase = ''
      mv coverage $out;
      export MOTOKO_CORE="${core-src}"
      mkdir -p $out/nix-support
      echo "report coverage $out index.html" >> $out/nix-support/hydra-build-products
    '';
  };

in
fix_names
  {
    run-release = test_subdir "run" [ moc ];
    run-debug = snty_subdir "run" [ moc ];
    run-eop-release = enhanced_orthogonal_persistence_subdir "run" [ moc ];
    run-eop-debug = snty_enhanced_orthogonal_persistence_subdir "run" [ moc ];
    drun-release = test_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-debug = snty_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-compacting-gc = snty_compacting_gc_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-generational-gc = snty_generational_gc_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-incremental-gc = snty_incremental_gc_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-eop-release = enhanced_orthogonal_persistence_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    drun-eop-debug = snty_enhanced_orthogonal_persistence_subdir "run-drun" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    fail = test_subdir "fail" [ moc ];
    repl = test_subdir "repl" [ moc ];
    ld = test_subdir "ld" ([ mo-ld ] ++ ldTestDeps);
    ld-eop = enhanced_orthogonal_persistence_subdir "ld" ([ mo-ld ] ++ ldTestDeps);
    idl = test_subdir "idl" [ didc ];
    mo-idl = test_subdir "mo-idl" [ moc didc ];
    mo-idl-eop = enhanced_orthogonal_persistence_subdir "mo-idl" [ moc didc ];
    trap = test_subdir "trap" [ moc ];
    trap-eop = enhanced_orthogonal_persistence_subdir "trap" [ moc ];
    run-deser = test_subdir "run-deser" [ deser ];
    perf = perf_subdir false "perf" [ moc test-runner pkgs.pocket-ic.server pkgs.cacert ];
    # TODO: profiling-graph is excluded because the underlying parity_wasm is deprecated and does not support passive data segments and memory64.
    inherit qc unit candid coverage;
  }
  // pkgs.lib.optionalAttrs
  (pkgs.system == accept-bench)
  (fix_names { bench = perf_subdir true "bench" [ moc test-runner pkgs.pocket-ic.server pkgs.ic-wasm pkgs.cacert ]; })
