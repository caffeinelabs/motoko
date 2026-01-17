{
  description = "The Motoko compiler";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    flake-utils.url = "github:numtide/flake-utils";

    nix-update-flake.url = "github:Mic92/nix-update/09aadb5d6d9e1fc57df0b61def4bdd8b43ea47a1";
    nix-update-flake.inputs.nixpkgs.follows = "nixpkgs";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    esm = {
      url = "https://registry.npmjs.org/esm/-/esm-3.2.25.tgz";
      flake = false;
    };

    candid-src = {
      url = "github:dfinity/candid";
      flake = false;
    };
    ic-wasm-src = {
      url = "github:dfinity/ic-wasm";
      flake = false;
    };
    libtommath-src = {
      url = "github:libtom/libtommath/v1.2.0";
      flake = false;
    };
    motoko-base-src = {
      url = "github:caffeinelabs/motoko-base/next-moc";
      flake = false;
    };
    motoko-core-src = {
      url = "github:caffeinelabs/motoko-core";
      flake = false;
    };
    motoko-matchers-src = {
      url = "github:kritzcreek/motoko-matchers/5ba5f52bd9a5649dedf5e2a1ccd55d98ed7ff982";
      flake = false;
    };
    ocaml-vlq-src = {
      url = "github:flowtype/ocaml-vlq";
      flake = false;
    };
    wasm-spec-src = {
      url = "github:WebAssembly/spec/opam-1.1.1";
      flake = false;
    };
    ocaml-recovery-parser-src = {
      url = "github:serokell/ocaml-recovery-parser";
      flake = false;
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , nix-update-flake
    , rust-overlay
    , esm
    , candid-src
    , ic-wasm-src
    , libtommath-src
    , motoko-base-src
    , motoko-core-src
    , motoko-matchers-src
    , ocaml-vlq-src
    , wasm-spec-src
    , ocaml-recovery-parser-src
    }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import ./nix/pkgs.nix {
        inherit nixpkgs system rust-overlay;
        sources = {
          inherit
            candid-src
            ic-wasm-src
            libtommath-src
            motoko-base-src
            motoko-core-src
            motoko-matchers-src
            ocaml-vlq-src
            wasm-spec-src
            ocaml-recovery-parser-src;
        };
      };

      llvmEnv = ''
        # When compiling to wasm, we want to have more control over the flags,
        # so we do not use the nix-provided wrapper in clang
        export WASM_CLANG="clang-19"
        export WASM_LD=wasm-ld
        # because we use the unwrapped clang, we have to pass in some flags/paths
        # that otherwise the wrapped clang would take care for us
        export WASM_CLANG_LIB="${pkgs.llvmPackages_19.clang-unwrapped.lib}"

        # When compiling natively, we want to use `clang` (which is a nixpkgs
        # provided wrapper that sets various include paths etc).
        # But for some reason it does not handle building for Wasm well, so
        # there we use plain clang-19. There is no stdlib there anyways.
        export CLANG="${pkgs.clang_19}/bin/clang"
      '';

      rts = import ./nix/rts.nix { inherit pkgs llvmEnv; };

      commonBuildInputs = pkgs: with pkgs; [ dune_3 obelisk perl removeReferencesTo ] ++ (with ocamlPackages; [
        ocaml
        checkseum
        findlib
        menhir
        menhirLib
        menhirSdk
        ocaml-recovery-parser
        cow
        num
        stdint
        wasm_1
        vlq
        zarith
        yojson
        ppxlib
        ppx_blob
        ppx_inline_test
        ppx_expect
        bisect_ppx
        uucp
      ]);

      moPackages = officialRelease: import ./nix/mo-packages.nix { inherit pkgs commonBuildInputs rts officialRelease; };
      releaseMoPackages = moPackages true;
      debugMoPackages = moPackages false;
      buildableMoPackages = moPackages: { inherit (moPackages) moc mo-ld mo-doc didc deser; };
      buildableReleaseMoPackages = buildableMoPackages releaseMoPackages;
      buildableDebugMoPackages = buildableMoPackages debugMoPackages;

      # Common cargo lock configuration for test-runner packages.
      test-runner-cargo-lock = {
        lockFile = ./test-runner/Cargo.lock;
        outputHashes = {
          "pocket-ic-10.0.0" = "sha256-Y71hDHsqxcDlUzKBP9fd9HyO1L51kqwTbIyTrGMRftk=";
        };
      };

      # Define test-runner package.
      test-runner = pkgs.rustPlatform-stable.buildRustPackage {
        pname = "test-runner";
        version = "0.1.0";
        src = ./test-runner;
        cargoLock = test-runner-cargo-lock;
        buildInputs = [
          pkgs.pocket-ic.server
        ];
        # Make pocket-ic available during tests.
        checkInputs = [ pkgs.pocket-ic.server pkgs.cacert ];
        nativeCheckInputs = [ pkgs.pocket-ic.server ];

        POCKET_IC_BIN = "${pkgs.pocket-ic.server}/bin/pocket-ic-server";
        SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";

        doCheck = true;
      };

      tests = import ./nix/tests.nix {
        inherit pkgs llvmEnv esm commonBuildInputs debugMoPackages test-runner;
      };

      filterTests = type:
        pkgs.lib.mapAttrsToList (_name: drv: drv) (pkgs.lib.filterAttrs
          (name: _drv:
            let
              matchDebug = builtins.match ".*-debug$" name;
              matchRelease = builtins.match ".*-release$" name;
              matchGC = builtins.match ".*-gc$" name;
              matchPerf = builtins.match ".*(bench|perf)$" name;
              # Common tests are those that do not match -debug, -release, -gc, or -bench, -perf.
              matchCommon = matchDebug == null &&
                matchRelease == null &&
                matchGC == null &&
                matchPerf == null;
            in
            {
              debug = matchDebug != null;
              release = matchRelease != null;
              gc = matchGC != null;
              common = matchCommon;
            }.${type})
          tests);

      base-src = pkgs.symlinkJoin {
        name = "base-src";
        paths = [ "${pkgs.sources.motoko-base-src}/src" ];
      };

      core-src = pkgs.symlinkJoin {
        name = "core-src";
        paths = [ "${pkgs.sources.motoko-core-src}/src" ];
      };

      js = import ./nix/moc.js.nix { inherit pkgs commonBuildInputs rts; };

      docs = import ./nix/docs.nix { inherit pkgs js base-src core-src; };

      checks = {
        check-formatting = import ./nix/check-formatting.nix { inherit pkgs; };
        check-rts-formatting = import ./nix/check-rts-formatting.nix { inherit pkgs; };
        check-grammar = import ./nix/check-grammar.nix { inherit pkgs; };
        check-error-codes = import ./nix/check-error-codes.nix { inherit pkgs; };
      };

      nix-update = nix-update-flake.packages.${system}.default;

      shell = import ./nix/shell.nix {
        inherit pkgs nix-update base-src core-src llvmEnv esm commonBuildInputs rts js debugMoPackages docs test-runner;
        inherit (checks) check-rts-formatting;
      };

      common-constituents = rec {
        samples = import ./nix/samples.nix { inherit pkgs; inherit (debugMoPackages) moc; };
        # TODO: Re-enable base tests once we recalibrate them so they
        # don't OOM anymore.
        # base-tests = import ./nix/base-tests.nix { inherit pkgs; inherit (debugMoPackages) moc; };
        base-doc = import ./nix/base-doc.nix { inherit pkgs; inherit (debugMoPackages) mo-doc; };
        report-site = import ./nix/report-site.nix { inherit pkgs base-doc docs; inherit (tests) coverage; };

        inherit rts base-src core-src docs shell;
      };
    in
    {
      packages = checks // common-constituents // rec {
        release = buildableReleaseMoPackages;
        debug = buildableDebugMoPackages;

        inherit nix-update tests js test-runner;

        inherit (pkgs) nix-build-uncached ic-wasm pocket-ic;

        # Get pocket-ic server.
        pocket-ic-server = pkgs.pocket-ic.server;



        # Platform-specific release files.
        release-files-ubuntu-latest = import ./nix/release-files-ubuntu-latest.nix { inherit self pkgs; };
        "release-files-ubuntu-24.04-arm" = import ./nix/release-files-ubuntu-24.04-arm.nix { inherit self pkgs; };
        release-files-macos-15-intel = import ./nix/release-files-macos-15-intel.nix { inherit self pkgs; };
        release-files-macos-latest = import ./nix/release-files-macos-latest.nix { inherit self pkgs; };

        # Common tests version - includes non-GC, non-release/debug specific tests.
        common-tests = pkgs.releaseTools.aggregate {
          name = "common-tests";
          constituents = filterTests "common"; # Only include common tests.
        };

        # GC tests version - only includes GC tests.
        gc-tests = pkgs.releaseTools.aggregate {
          name = "gc-tests";
          constituents = filterTests "gc"; # Only include GC tests.
        };

        # Release version - excludes debug tests.
        release-systems-go = pkgs.releaseTools.aggregate {
          name = "release-systems-go";
          constituents =
            pkgs.lib.attrValues common-constituents ++
              pkgs.lib.attrValues checks ++
              pkgs.lib.attrValues buildableReleaseMoPackages ++
              filterTests "release" # Only include release tests.
              ++ builtins.attrValues js;
        };

        # Debug version - only includes debug tests.
        debug-systems-go = pkgs.releaseTools.aggregate {
          name = "debug-systems-go";
          constituents =
            pkgs.lib.attrValues common-constituents ++
              pkgs.lib.attrValues checks ++
              pkgs.lib.attrValues buildableDebugMoPackages ++
              filterTests "debug"  # Only include debug tests
              ++ builtins.attrValues js;
        };

        inherit (debug) moc;

        default = release-systems-go;
      };

      checks = checks // tests;

      devShells.default = shell;

      formatter = pkgs.writeShellScriptBin "formatter" ''
        if [[ $# = 0 ]]; then set -- .; fi
        exec "${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt" "$@"
      '';

      # Used to list all buildable targets for the `nixb` command available inside nix develop
      targets =
        let
          # TODO: remove once motoko's nixpkgs has: https://github.com/NixOS/nixpkgs/commit/e5ac84e29c4b04ea5934d166f270dd6516ad76c7
          inherit (builtins) isAttrs concatMap attrNames;
          mapAttrsToListRecursiveCond =
            pred: f: set:
            let
              mapRecursive =
                path: value: if isAttrs value && pred path value then recurse path value else [ (f path value) ];
              recurse = path: set: concatMap (name: mapRecursive (path ++ [ name ]) set.${name}) (attrNames set);
            in
            recurse [ ] set;
        in
        mapAttrsToListRecursiveCond
          (path: as: !(pkgs.lib.isDerivation as))
          (path: _drv: pkgs.lib.concatStringsSep "." path)
          self.packages.${system};

    });
}
