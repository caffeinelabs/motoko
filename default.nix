{
  replay ? 0,
  system ? builtins.currentSystem,
  officialRelease ? false,
}:

let nixpkgs = import ./nix { inherit system; }; in

let releaseVersion = import nix/releaseVersion.nix { pkgs = nixpkgs; inherit officialRelease; }; in

let stdenv = nixpkgs.stdenv; in

let subpath = import ./nix/gitSource.nix; in

let ic-hs-pkgs = import nixpkgs.sources.ic-hs { inherit (nixpkgs) system; }; in
let ic-ref-run =
  # copy out the binary, to remove dependencies on the libraries
  nixpkgs.runCommandNoCC "ic-ref-run" {} ''
      mkdir -p $out/bin
      cp ${ic-hs-pkgs.ic-hs}/bin/ic-ref-run $out/bin
  ''; in

let haskellPackages = nixpkgs.haskellPackages.override {
      overrides = import nix/haskell-packages.nix nixpkgs subpath;
    }; in
let
  rtsBuildInputs = with nixpkgs; [
    # pulls in clang (wrapped) and clang-13 (unwrapped)
    llvmPackages_13.clang
    # pulls in wasm-ld
    llvmPackages_13.lld
    llvmPackages_13.bintools
    rustc-nightly
    cargo-nightly
    wasmtime
    rust-bindgen
    python3
    emscripten
  ] ++ pkgs.lib.optional pkgs.stdenv.isDarwin [
    libiconv
  ];

  llvmEnv = ''
    # When compiling to wasm, we want to have more control over the flags,
    # so we do not use the nix-provided wrapper in clang
    export WASM_CLANG="clang-13"
    export WASM_LD=wasm-ld
    # because we use the unwrapped clang, we have to pass in some flags/paths
    # that otherwise the wrapped clang would take care for us
    export WASM_CLANG_LIB="${nixpkgs.llvmPackages_13.clang-unwrapped.lib}"

    # When compiling natively, we want to use `clang` (which is a nixpkgs
    # provided wrapper that sets various include paths etc).
    # But for some reason it does not handle building for Wasm well, so
    # there we use plain clang-13. There is no stdlib there anyways.
    export CLANG="${nixpkgs.clang_13}/bin/clang"
  '';
in

# When building for linux (but not in nix-shell) we build statically
let is_static = !nixpkgs.stdenv.isDarwin; in

let staticpkgs = if is_static then nixpkgs.pkgsMusl else nixpkgs; in

# This branches on the pkgs, which is either
# normal nixpkgs (nix-shell, darwin)
# nixpkgs.pkgsMusl for static building (release builds)
let commonBuildInputs = pkgs:
  [
    pkgs.dune_3
    pkgs.ocamlPackages.ocaml
    pkgs.ocamlPackages.atdgen
    pkgs.ocamlPackages.checkseum
    pkgs.ocamlPackages.findlib
    pkgs.ocamlPackages.menhir
    pkgs.ocamlPackages.menhirLib
    pkgs.ocamlPackages.cow
    pkgs.ocamlPackages.num
    pkgs.ocamlPackages.stdint
    pkgs.ocamlPackages.wasm
    pkgs.ocamlPackages.vlq
    pkgs.ocamlPackages.zarith
    pkgs.ocamlPackages.yojson
    pkgs.ocamlPackages.ppxlib
    pkgs.ocamlPackages.ppx_blob
    pkgs.ocamlPackages.ppx_inline_test
    pkgs.ocamlPackages.bisect_ppx
    pkgs.ocamlPackages.uucp
    pkgs.obelisk
    pkgs.perl
    pkgs.removeReferencesTo
  ]; in

let ocaml_exe = name: bin: rts:
  let
    profile =
      if is_static
      then "release-static"
      else "release";
  in
    staticpkgs.stdenv.mkDerivation {
      inherit name;

      allowedRequisites = [];

      src = subpath ./src;

      buildInputs = commonBuildInputs staticpkgs;

      MOTOKO_RELEASE = releaseVersion;

      extraDuneOpts = "";

      # we only need to include the wasm statically when building moc, not
      # other binaries
      buildPhase = ''
        patchShebangs .
      '' + nixpkgs.lib.optionalString (rts != null)''
        ./rts/gen.sh ${rts}/rts
      '' + ''
        make DUNE_OPTS="--display=short --profile ${profile} $extraDuneOpts" ${bin}
      '';

      installPhase = ''
        mkdir -p $out/bin
        cp --verbose --dereference ${bin} $out/bin
      '' + nixpkgs.lib.optionalString nixpkgs.stdenv.isDarwin ''
        # there are references to darwin system libraries
        # in the binaries. But curiously, we can remove them
        # an the binaries still work. They are essentially static otherwise.
        remove-references-to \
          -t ${nixpkgs.darwin.Libsystem} \
          -t ${nixpkgs.darwin.CF} \
          -t ${nixpkgs.libiconv} \
          $out/bin/*
      '' + ''
        # also, there is a refernece to /nix/store/…/share/menhir/standard.mly.
        # Let's remove that, too
        remove-references-to \
          -t ${staticpkgs.ocamlPackages.menhir} \
          $out/bin/*
        # sanity check
        $out/bin/* --help >/dev/null
      '';
    };

  musl-wasi-sysroot = stdenv.mkDerivation {
    name = "musl-wasi-sysroot";
    src = nixpkgs.sources.musl-wasi;
    phases = [ "unpackPhase" "installPhase" ];
    installPhase = ''
      make SYSROOT="$out" include_dirs
    '';
  };
in

rec {
  rts =
    let
      # Build Rust package cargo-vendor-tools
      cargoVendorTools = nixpkgs.rustPlatform.buildRustPackage rec {
        name = "cargo-vendor-tools";
        src = subpath "./rts/${name}/";
        cargoSha256 = "sha256-E6GTFvmZMjGsVlec7aH3QaizqIET6Dz8Csh0N1jeX+M=";
      };

      # Path to vendor-rust-std-deps, provided by cargo-vendor-tools
      vendorRustStdDeps = "${cargoVendorTools}/bin/vendor-rust-std-deps";

      # SHA256 of Rust std deps
      rustStdDepsHash = "sha256-A3WPIx+weu4wIYV7cweGkRxYGAPt7srxBAtMEyPOkhI=";

      # Vendor directory for Rust std deps
      rustStdDeps = nixpkgs.stdenvNoCC.mkDerivation {
        name = "rustc-std-deps";

        nativeBuildInputs = with nixpkgs; [
          curl
        ];

        buildCommand = ''
          mkdir $out
          cd $out
          ${vendorRustStdDeps} ${nixpkgs.rustc-nightly} .
        '';

        outputHash = rustStdDepsHash;
        outputHashAlgo = "sha256";
        outputHashMode = "recursive";
      };

      # Vendor tarball of the RTS
      rtsDeps = nixpkgs.rustPlatform.fetchCargoTarball {
        name = "motoko-rts-deps";
        src = subpath ./rts;
        sourceRoot = "rts/motoko-rts-tests";
        sha256 = "sha256-prLZVOWV3BFb8/nKHyqZw8neJyBu1gs5d0D56DsDV2o=";
        copyLockfile = true;
      };

      # Unpacked RTS deps
      rtsDepsUnpacked = nixpkgs.stdenvNoCC.mkDerivation {
        name = rtsDeps.name + "-unpacked";
        buildCommand = ''
          tar xf ${rtsDeps}
          mv *.tar.gz $out
        '';
      };

      # All dependencies needed to build the RTS, including Rust std deps, to
      # allow `cargo -Zbuild-std`. (rust-lang/wg-cargo-std-aware#23)
      allDeps = nixpkgs.symlinkJoin {
        name = "merged-rust-deps";
        paths = [
          rtsDepsUnpacked
          rustStdDeps
        ];
      };
    in

    stdenv.mkDerivation {
      name = "moc-rts";

      src = subpath ./rts;

      nativeBuildInputs = [ nixpkgs.makeWrapper nixpkgs.removeReferencesTo nixpkgs.cacert ];

      buildInputs = rtsBuildInputs;

      preBuild = ''
        export CARGO_HOME=$PWD/cargo-home

        # This replicates logic from nixpkgs’ pkgs/build-support/rust/default.nix
        mkdir -p $CARGO_HOME
        echo "Using vendored sources from ${rtsDeps}"
        unpackFile ${allDeps}
        cat > $CARGO_HOME/config <<__END__
          [source."crates-io"]
          "replace-with" = "vendored-sources"

          [source."vendored-sources"]
          "directory" = "$(stripHash ${allDeps})"
        __END__

        
        ${llvmEnv}
        export TOMMATHSRC=${nixpkgs.sources.libtommath}
        export MUSLSRC=${nixpkgs.sources.musl-wasi}/libc-top-half/musl
        export MUSL_WASI_SYSROOT=${musl-wasi-sysroot}
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
          -t ${nixpkgs.rustc-nightly} \
          -t ${rtsDeps} \
          -t ${rustStdDeps} \
          $out/rts/mo-rts-non-incremental.wasm $out/rts/mo-rts-non-incremental-debug.wasm
        remove-references-to \
          -t ${nixpkgs.rustc-nightly} \
          -t ${rtsDeps} \
          -t ${rustStdDeps} \
          $out/rts/mo-rts-incremental.wasm $out/rts/mo-rts-incremental-debug.wasm
        remove-references-to \
          -t ${nixpkgs.rustc-nightly} \
          -t ${rtsDeps} \
          -t ${rustStdDeps} \
          $out/rts/mo-rts-eop.wasm $out/rts/mo-rts-eop-debug.wasm
      '';

      allowedRequisites = [];
    };

  moc = ocaml_exe "moc" "moc" rts;
  mo-ld = ocaml_exe "mo-ld" "mo-ld" null;
  mo-ide = ocaml_exe "mo-ide" "mo-ide" null;
  mo-doc = ocaml_exe "mo-doc" "mo-doc" null;
  didc = ocaml_exe "didc" "didc" null;
  deser = ocaml_exe "deser" "deser" null;
  candid-tests = ocaml_exe "candid-tests" "candid-tests" null;

  # executable built with coverage:
  coverage_bins = builtins.listToAttrs (nixpkgs.lib.flip map [moc mo-ld didc deser ] (drv:
    { name = drv.name;
      value = drv.overrideAttrs(old : {
        name = "${old.name}-coverage";
        extraDuneOpts="--instrument-with bisect_ppx";
        installPhase = old.installPhase + ''
          # The coverage report needs access to sources, including generated ones
          # like _build/parser.ml
          mkdir $out/src
          find -name \*.ml -print0 | xargs -0 cp -t $out/src --parents
        '';
        allowedRequisites = null;
      });
    }));

  # “our” Haskell packages
  inherit (haskellPackages) lsp-int qc-motoko;

  inherit ic-ref-run;

  tests = let
    testDerivationArgs = {
      # by default, an empty source directory. how to best get an empty directory?
      src = builtins.path { name = "empty"; path = ./nix; filter = p: t: false; };
      phases = "unpackPhase checkPhase installPhase";
      doCheck = true;
      installPhase = "touch $out";
    };

    testDerivationDeps =
      (with nixpkgs; [ wabt bash perl getconf moreutils nodejs-18_x ]) ++
      [ filecheck wasmtime ];


    # extra deps for test/ld
    ldTestDeps =
      with nixpkgs; [ llvmPackages_13.bintools llvmPackages_13.clang ];

    testDerivation = args:
      stdenv.mkDerivation (testDerivationArgs // args);

    # we test each subdirectory of test/ in its own derivation with
    # cleaner dependencies, for more parallelism, more caching
    # and better feedback about what aspect broke
    # so include from test/ only the common files, plus everything in test/${dir}/
    test_src = dir:
      with nixpkgs.lib;
      cleanSourceWith {
        filter = path: type:
          let relPath = removePrefix (toString ./test + "/") (toString path); in
          type != "directory" || hasPrefix "${dir}/" "${relPath}/";
        src = subpath ./test;
        name = "test-${dir}-src";
      };

    test_subdir = dir: deps:
      testDerivation {
        src = test_src dir;
        buildInputs = deps ++ testDerivationDeps;

        checkPhase = ''
            patchShebangs .
            ${llvmEnv}
            export ESM=${nixpkgs.sources.esm}
            export VIPER_SERVER=${viperServer}
            type -p moc && moc --version
            make -C ${dir}
          '';
      };

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

    perf_subdir = dir: deps:
      (test_subdir dir deps).overrideAttrs (args: {
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
            echo "perf stats do not include gas. change in drun output format?" >&2
            exit 1
          fi
        '';
      });

    qc = testDerivation {
      buildInputs =
        [ moc wasmtime haskellPackages.qc-motoko nixpkgs.drun ];
      checkPhase = ''
        export LANG=C.utf8 # for haskell
        qc-motoko${nixpkgs.lib.optionalString (replay != 0)
            " --quickcheck-replay=${toString replay}"}
      '';
    };

    lsp = testDerivation {
      src = subpath ./test/lsp-int-test-project;
      buildInputs = [ moc haskellPackages.lsp-int ];
      checkPhase = ''
        echo running lsp-int
        export LANG=C.utf8 # for haskell
        lsp-int ${mo-ide}/bin/mo-ide .
      '';
    };

    unit = testDerivation {
      src = subpath ./src;
      buildInputs = commonBuildInputs nixpkgs;
      checkPhase = ''
        patchShebangs .
        make DUNE_OPTS="--display=short" unit-tests
      '';
      installPhase = ''
        touch $out
      '';
    };

    candid = testDerivation {
      buildInputs = [ moc wasmtime candid-tests ];
      checkPhase = ''
        candid-tests -i ${nixpkgs.sources.candid}/test
      '';
    };

    # wasm-profiler is not compatible with passive data segments and memory64
    # profiling-graphs = testDerivation {
    #  src = test_src "perf";
    #  buildInputs =
    #    (with nixpkgs; [ perl wabt wasm-profiler-instrument wasm-profiler-postproc flamegraph-bin ]) ++
    #    [ moc nixpkgs.drun ];
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
      src = subpath ./test;
      buildInputs =
          builtins.attrValues coverage_bins ++
          [ nixpkgs.ocamlPackages.bisect_ppx ] ++
          testDerivationDeps ++
          ldTestDeps;

      checkPhase = ''
          patchShebangs .
          ${llvmEnv}
          export ESM=${nixpkgs.sources.esm}
          export SOURCE_PATHS="${
            builtins.concatStringsSep " " (map (d: "${d}/src") (builtins.attrValues coverage_bins))
          }"
          type -p moc && moc --version
          make coverage
      '';
      installPhase = ''
        mv coverage $out;
        mkdir -p $out/nix-support
        echo "report coverage $out index.html" >> $out/nix-support/hydra-build-products
      '';
    };

  in fix_names ({
      run        = test_subdir "run"        [ moc ] ;
      run-dbg    = snty_subdir "run"        [ moc ] ;
      run-eop-release = enhanced_orthogonal_persistence_subdir "run" [ moc ];
      run-eop-debug = snty_enhanced_orthogonal_persistence_subdir "run" [ moc ];
      # ic-ref-run = test_subdir "run-drun"   [ moc ic-ref-run ];
      drun       = test_subdir "run-drun"   [ moc nixpkgs.drun ];
      drun-dbg   = snty_subdir "run-drun"   [ moc nixpkgs.drun ];
      drun-compacting-gc = snty_compacting_gc_subdir "run-drun" [ moc nixpkgs.drun ] ;
      drun-generational-gc = snty_generational_gc_subdir "run-drun" [ moc nixpkgs.drun ] ;
      drun-incremental-gc = snty_incremental_gc_subdir "run-drun" [ moc nixpkgs.drun ] ;
      drun-eop-release = enhanced_orthogonal_persistence_subdir "run-drun" [ moc nixpkgs.drun ] ;
      drun-eop-debug = snty_enhanced_orthogonal_persistence_subdir "run-drun" [ moc nixpkgs.drun ] ;
      fail       = test_subdir "fail"       [ moc ];
      fail-eop   = enhanced_orthogonal_persistence_subdir "fail"       [ moc ];
      repl       = test_subdir "repl"       [ moc ];
      repl-eop   = enhanced_orthogonal_persistence_subdir "repl"       [ moc ];
      ld         = test_subdir "ld"         ([ mo-ld ] ++ ldTestDeps);
      ld-eop     = enhanced_orthogonal_persistence_subdir "ld" ([ mo-ld ] ++ ldTestDeps);
      idl        = test_subdir "idl"        [ didc ];
      mo-idl     = test_subdir "mo-idl"     [ moc didc ];
      mo-idl-eop = enhanced_orthogonal_persistence_subdir "mo-idl" [ moc didc ];
      trap       = test_subdir "trap"       [ moc ];
      trap-eop   = enhanced_orthogonal_persistence_subdir "trap" [ moc ];
      run-deser  = test_subdir "run-deser"  [ deser ];
      perf       = perf_subdir "perf"       [ moc nixpkgs.drun ];
      bench      = perf_subdir "bench"      [ moc nixpkgs.drun ic-wasm ];
      viper      = test_subdir "viper"      [ moc nixpkgs.which nixpkgs.openjdk nixpkgs.z3_4_12 ];
      # TODO: profiling-graph is excluded because the underlying partity_wasm is deprecated and does not support passive data segments and memory64.
      inherit qc lsp unit candid coverage;
    }) // { recurseForDerivations = true; };

  samples = stdenv.mkDerivation {
    name = "samples";
    src = subpath ./samples;
    buildInputs = [ moc ];
    buildPhase = ''
      patchShebangs .
      make all
    '';
    installPhase = ''
      touch $out
    '';
  };

  js =
    let mk = n:
      stdenv.mkDerivation {
        name = "${n}.js";
        src = subpath ./src;
        buildInputs = commonBuildInputs nixpkgs ++ [
          nixpkgs.ocamlPackages.js_of_ocaml
          nixpkgs.ocamlPackages.js_of_ocaml-ppx
          nixpkgs.nodejs-18_x
          nixpkgs.nodePackages.terser
        ];
        buildPhase = ''
          patchShebangs .
          '' + nixpkgs.lib.optionalString (rts != null)''
          ./rts/gen.sh ${rts}/rts/
          '' + ''
          make DUNE_OPTS="--profile=release" ${n}.js
          terser ${n}.js -o ${n}.min.js -c -m
        '';
        installPhase = ''
          mkdir -p $out
          mkdir -p $out/bin
          cp --verbose --dereference ${n}.js $out/bin
          cp --verbose --dereference ${n}.min.js $out/bin
        '';
        doInstallCheck = true;
        test = ./test + "/test-${n}.js";
        installCheckPhase = ''
          NODE_PATH=$out/bin node --experimental-wasm-memory64 $test
        '';
      };
    in
    {
      moc = mk "moc";
      moc_interpreter = mk "moc_interpreter";
      didc = mk "didc";
      recurseForDerivations = true;
    };

  inherit (nixpkgs) drun wabt wasmtime wasm nix-update;

  filecheck = nixpkgs.runCommandNoCC "FileCheck" {} ''
    mkdir -p $out/bin
    cp ${nixpkgs.llvm}/bin/FileCheck $out/bin
  '';

  ic-wasm =
    nixpkgs.rustPlatform.buildRustPackage {
      name = "ic-wasm";
      src = nixpkgs.sources.ic-wasm;
      cargoSha256 = "sha256-a8iN/lTEVqdmogsSlT3+v3nivSG5VRhOz4/trmAsZLY=";
      doCheck = false;
      patchPhase = ''
        mkdir -p .cargo
        cat > .cargo/config.toml << EOF
[target.x86_64-apple-darwin]
rustflags = [ "-C", "linker=c++" ]

[target.aarch64-apple-darwin]
rustflags = [ "-C", "linker=c++" ]
EOF
      '';
    };

  # gitMinimal is used by nix/gitSource.nix; building it here warms the nix cache
  inherit (nixpkgs) gitMinimal;

  docs = stdenv.mkDerivation {
    name = "docs";
    src = subpath ./doc;
    buildInputs = with nixpkgs; [ pandoc bash gitMinimal ];

    buildPhase = ''
      patchShebangs .
      export HOME=$PWD
      export MOC_JS=${js.moc}/bin/moc.js
      export MOTOKO_BASE=${base-src}
      make
    '';

    installPhase = ''
      mkdir -p $out
      mv overview-slides.html $out/
      mv html $out/
      mkdir -p $out/nix-support
      echo "report guide $out html/motoko.html" >> $out/nix-support/hydra-build-products
      echo "report slides $out overview-slides.html" >> $out/nix-support/hydra-build-products
    '';
  };

  check-formatting = stdenv.mkDerivation {
    name = "check-formatting";
    buildInputs = [ nixpkgs.ocamlformat ];
    src = subpath ./src;
    doCheck = true;
    phases = "unpackPhase checkPhase installPhase";
    installPhase = "touch $out";
    checkPhase = ''
      ocamlformat --check languageServer/*.{ml,mli} docs/*.{ml,mli}
    '';
  };

  check-rts-formatting = stdenv.mkDerivation {
    name = "check-rts-formatting";
    buildInputs = [ nixpkgs.cargo-nightly nixpkgs.rustfmt ];
    src = subpath ./rts;
    doCheck = true;
    phases = "unpackPhase checkPhase installPhase";
    checkPhase = ''
      echo "If this fails, run `make -C rts format`"
      cargo fmt --verbose --manifest-path motoko-rts/Cargo.toml -- --check
      cargo fmt --verbose --manifest-path motoko-rts-tests/Cargo.toml -- --check
    '';
    installPhase = "touch $out";
  };

  base-src = nixpkgs.symlinkJoin {
    name = "base-src";
    paths = "${nixpkgs.sources.motoko-base}/src";
  };

  base-tests = stdenv.mkDerivation {
    name = "base-tests";
    src = nixpkgs.sources.motoko-base;
    phases = "unpackPhase checkPhase installPhase";
    doCheck = true;
    installPhase = "touch $out";
    checkInputs = [
      nixpkgs.wasmtime
      moc
    ];
    checkPhase = ''
      make MOC=moc VESSEL_PKGS="--package matchers ${nixpkgs.sources.motoko-matchers}/src" -C test
    '';
  };

  guide-examples-tc =  stdenv.mkDerivation {
    name = "guid-examples-tc";
    src = subpath ./doc/md/examples;
    phases = "unpackPhase checkPhase installPhase";
    doCheck = true;
    MOTOKO_BASE = base-src;
    installPhase = "touch $out";
    checkInputs = [
      moc
    ];
    checkPhase = ''
      patchShebangs .
      ./check.sh
    '';
  };

  base-doc = stdenv.mkDerivation {
    name = "base-doc";
    src = nixpkgs.sources.motoko-base;
    phases = "unpackPhase buildPhase installPhase";
    doCheck = true;
    buildInputs = [ mo-doc ];
    buildPhase = ''
      mo-doc
    '';
    installPhase = ''
      mkdir -p $out
      cp -rv docs/* $out/

      mkdir -p $out/nix-support
      echo "report docs $out index.html" >> $out/nix-support/hydra-build-products
    '';
  };

  report-site = nixpkgs.runCommandNoCC "report-site" {
    buildInputs = [ nixpkgs.tree ];
  } ''
    mkdir -p $out
    ln -s ${base-doc} $out/base-doc
    ln -s ${docs} $out/docs
    ln -s ${tests.coverage} $out/coverage
    cd $out;
    # generate a simple index.html, listing the entry points
    ( echo docs/overview-slides.html;
      echo docs/html/motoko.html;
      echo base-doc/
      echo coverage/ ) | \
      tree -H . -l --fromfile -T "Motoko build reports" > index.html
  '';

  check-generated = nixpkgs.runCommandNoCC "check-generated" {
      nativeBuildInputs = [ nixpkgs.diffutils ];
      expected = import ./nix/generate.nix { pkgs = nixpkgs; };
      dir = ./nix/generated;
    } ''
      diff -r -U 3 $expected $dir
      touch $out
    '';

  # Checks that doc/md/examples/grammar.txt is up-to-date
  check-grammar = stdenv.mkDerivation {
      name = "check-grammar";
      src = subpath ./src/gen-grammar;
      phases = "unpackPhase buildPhase installPhase";
      buildInputs = [ nixpkgs.diffutils nixpkgs.bash nixpkgs.obelisk ];
      buildPhase = ''
        patchShebangs .
        ./gen-grammar.sh ${./src/mo_frontend/parser.mly} > expected
        echo "If the following fails, please run:"
        echo "nix-shell --command 'make -C src grammar'"
        diff -r -U 3 ${./doc/md/examples/grammar.txt} expected
        echo "ok, all good"
      '';
      installPhase = ''
        touch $out
      '';
    };

  check-error-codes = stdenv.mkDerivation {
      name = "check-error-codes";
      src = subpath ./test;
      phases = "unpackPhase buildPhase installPhase";
      buildInputs = [ nixpkgs.python3 ];
      buildPhase = ''
      patchShebangs .
      ./check-error-codes.py ${./src/lang_utils/error_codes.ml}
      '';
      installPhase = ''
        touch $out
      '';
  };

  all-systems-go = nixpkgs.releaseTools.aggregate {
    name = "all-systems-go";
    constituents = [
      moc
      mo-ide
      mo-doc
      didc
      deser
      samples
      rts
      base-src
      base-tests
      base-doc
      docs
      report-site
      # ic-ref-run
      shell
      check-formatting
      check-rts-formatting
      check-generated
      check-grammar
      check-error-codes
    ] ++
    builtins.attrValues tests
    ++ builtins.attrValues js
    ;
  };

  viperServer = nixpkgs.fetchurl {
    url = https://github.com/viperproject/viperserver/releases/download/v.22.11-release/viperserver.jar;
    sha256 = "sha256-debC8ZpbIjgpEeISCISU0EVySJvf+WsUkUaLuJ526wA=";
  };

  shell = nixpkgs.mkShell {
    name = "motoko-shell";

    #
    # Since building moc, and testing it, are two different derivations in we
    # have to create a fake derivation for `nix-shell` that commons up the
    # build dependencies of the two to provide a build environment that offers
    # both, while not actually building `moc`
    #
    propagatedBuildInputs =
      let dont_build =
        [ moc mo-ld didc deser candid-tests ] ++
        builtins.attrValues coverage_bins;
      in with nixpkgs;
      [ ic-wasm ] ++
      lib.lists.unique (builtins.filter (i: !(builtins.elem i dont_build)) (
        commonBuildInputs nixpkgs ++
        rts.buildInputs ++
        js.moc.buildInputs ++
        docs.buildInputs ++
        check-rts-formatting.buildInputs ++
        builtins.concatMap (d: d.buildInputs or []) (builtins.attrValues tests) ++
        [ ncurses
          ocamlPackages.merlin
          ocamlPackages.utop
          ocamlformat
          fswatch
          niv
          nix-update
          rlwrap # for `rlwrap moc`
          openjdk z3_4_12 # for viper dev
          difftastic
        ] ++ lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.Security
      ));

    shellHook = llvmEnv + ''
      # Include our wrappers in the PATH
      export PATH="${toString ./bin}:$PATH"
      # some cleanup of environment variables otherwise set by nix-shell
      # that would be confusing in interactive use
      unset XDG_DATA_DIRS
    '';
    ESM=nixpkgs.sources.esm;
    TOMMATHSRC = nixpkgs.sources.libtommath;
    MUSLSRC = "${nixpkgs.sources.musl-wasi}/libc-top-half/musl";
    MUSL_WASI_SYSROOT = musl-wasi-sysroot;
    LOCALE_ARCHIVE = nixpkgs.lib.optionalString stdenv.isLinux "${nixpkgs.glibcLocales}/lib/locale/locale-archive";
    MOTOKO_BASE = base-src;
    CANDID_TESTS = "${nixpkgs.sources.candid}/test";
    VIPER_SERVER = "${viperServer}";

    # allow building this as a derivation, so that hydra builds and caches
    # the dependencies of shell.
    #
    # Note that we are using propagatedBuildInputs above, not just buildInputs.
    # This means that the dependencies end up in the output path, in
    # /nix/store/13d…da6-motoko-shell/nix-support/propagated-build-inputs
    # so that after `nix-build -A shell` (or just `nix-build`) they are guaranteed
    # to be present in the local nix store (else this might just download an
    # empty build result path from the nix cache.)
    phases = ["installPhase" "fixupPhase"];
    installPhase = ''
      mkdir $out
    '';
    preferLocalBuild = true;
    allowSubstitutes = true;
  };
}
