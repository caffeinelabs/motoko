{ nixpkgs, system, rust-overlay, sources }: import nixpkgs {
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

    # wabt with WebAssembly/wabt#2744 patched in (return_call_indirect
    # + table64 validator fix). Patches nixpkgs's wabt source rather
    # than swapping it out so the third_party/* submodules nixpkgs
    # already fetches stay intact. Drop this overlay once #2744 lands
    # and propagates to nixpkgs.
    (self: super: {
      wabt = super.wabt.overrideAttrs (old: {
        patches = (old.patches or []) ++ [
          (super.fetchpatch {
            name = "wabt-pr-2744-return_call_indirect-table64.patch";
            url = "https://github.com/WebAssembly/wabt/pull/2744.patch";
            hash = "sha256-RzGaVgitOcv2KkHV1HT77A2WLPwJtBxYB9rS0u42tko=";
          })
        ];
        version = "${old.version}-pre-2744";
        __intentionallyOverridingVersion = true;
      });
    })
  ];
}
