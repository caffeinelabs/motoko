{ nixpkgs, system, rust-overlay, sources, wasmtime-src }: import nixpkgs {
  inherit system;
  overlays = [
    (self: super: { inherit sources; })

    # Selecting the ocaml version
    # Also update ocaml-version in src/*/.ocamlformat!
    (self: super: { ocamlPackages = self.ocaml-ng.ocamlPackages_5_4; })

    (self: super: rec {
      # Additional ocaml packages
      ocamlPackages = super.ocamlPackages // rec {

        ocaml-recovery-parser = super.ocamlPackages.buildDunePackage {
          pname = "ocaml-recovery-parser";
          version = "0.3.0";
          src = self.sources.ocaml-recovery-parser-src;
          buildInputs = with super.ocamlPackages; [
            menhirSdk
            menhirLib
            fix
            base
          ];
        };

        grace = super.ocamlPackages.buildDunePackage {
          pname = "grace";
          version = "0.3.0";
          src = self.sources.grace-src;
          buildInputs = with super.ocamlPackages; [
            dedent
            core
            ppx_jane
            iter
            uutf
            fmt
          ];
        };
      };
    }
    )

    # Rust Nightly
    rust-overlay.overlays.default
    (self: super: {
      # When you change the rust-nightly version,
      # make sure to change the rustStdDepsHash in ./rts.nix accordingly.
      rust-nightly = self.rust-bin.nightly."2026-05-04".default.override {
        extensions = [ "rust-src" ];
        targets = [ "wasm32-wasip1" ];
      };
    })

    # wasm-profiler
    (self: super: import ./wasm-profiler.nix self)

    # pocket-ic
    (self: super: { pocket-ic = import ./pocket-ic.nix self; })

    # ic-wasm
    (self: super: { ic-wasm = import ./ic-wasm.nix self; })

    # wasmtime: rebuild from upstream main (post-v45) so we get the
    # ctz/clz-in-brif simplify_skeleton fold (PR #13343) and the
    # planned i64 mirror that motoko's gabor/clz-msb-peephole peephole
    # relies on for tight x86_64 / aarch64 lowering.  cargoHash =
    # lib.fakeHash on first run — CI's nix-build error will tell us the
    # real hash, which we then commit.
    (self: super: {
      wasmtime = super.wasmtime.overrideAttrs (oa: {
        version = "main";
        src = wasmtime-src;
        cargoDeps = self.rustPlatform.importCargoLock {
          lockFile = "${wasmtime-src}/Cargo.lock";
          allowBuiltinFetchGit = true;
        };
      });
    })
  ];
}
