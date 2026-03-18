{ nixpkgs, system, rust-overlay, sources }: import nixpkgs {
  inherit system;
  overlays = [
    (self: super: { inherit sources; })

    # Selecting the ocaml version
    # Also update ocaml-version in src/*/.ocamlformat!
    (self: super: { ocamlPackages = self.ocaml-ng.ocamlPackages_4_14; })

    (self: super: {
      ocamlPackages = super.ocamlPackages.overrideScope (oself: osuper: {

        # downgrade wasm until we have support for 2.0.1
        # (https://github.com/dfinity/motoko/pull/3364)
        # nixos-25.11 switched wasm to buildDunePackage (2.0.2), but 1.1.1
        # is Makefile-based, so we define it from scratch.
        wasm_1 = self.stdenv.mkDerivation rec {
          pname = "ocaml${osuper.ocaml.version}-wasm";
          version = "1.1.1";
          src = self.sources.wasm-spec-src;
          nativeBuildInputs = with osuper; [ ocaml findlib ocamlbuild ];
          strictDeps = true;
          makeFlags = [ "-C" "interpreter" ];
          createFindlibDestdir = true;
          patchPhase = ''
            substituteInPlace ./interpreter/Makefile \
              --replace-fail "+a-4-27-42-44-45" "+a-4-27-42-44-45-70"
          '';
          postInstall = ''
            mkdir $out/bin
            cp -L interpreter/wasm $out/bin
          '';
        };

        ocaml-recovery-parser = osuper.buildDunePackage {
          pname = "ocaml-recovery-parser";
          version = "0.3.0";
          src = self.sources.ocaml-recovery-parser-src;
          buildInputs = with osuper; [
            menhirSdk
            menhirLib
            fix
            base
          ];
        };

        # macOS strip crashes on .wasm/.wat files in this package
        wasm_of_ocaml-compiler = osuper.wasm_of_ocaml-compiler.overrideAttrs (old: {
          dontStrip = true;
        });

        grace = osuper.buildDunePackage {
          pname = "grace";
          version = "0.3.0";
          src = self.sources.grace-src;
          buildInputs = with osuper; [
            dedent
            core
            ppx_jane
            iter
            uutf
            fmt
          ];
        };
      });
    })

    # Rust Nightly & Stable
    rust-overlay.overlays.default
    (self: super: {
      # When you change the rust-nightly version,
      # make sure to change the rustStdDepsHash in ./rts.nix accordingly.
      rust-nightly = self.rust-bin.nightly."2025-06-19".default.override {
        extensions = [ "rust-src" ];
        targets = [ "wasm32-wasip1" ];
      };

      rust-stable = self.rust-bin.stable."1.89.0".default;

      rustPlatform-stable = self.makeRustPlatform rec {
        rustc = self.rust-stable;
        cargo = rustc;
      };
    })

    # wasm-profiler
    (self: super: import ./wasm-profiler.nix self)

    # pocket-ic
    (self: super: { pocket-ic = import ./pocket-ic.nix self; })

    # ic-wasm
    (self: super: { ic-wasm = import ./ic-wasm.nix self; })
  ];
}
