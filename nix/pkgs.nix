{ nixpkgs, system, rust-overlay, sources }: import nixpkgs {
  inherit system;
  overlays = [
    (self: super: { inherit sources; })

    # Selecting the ocaml version
    # Also update ocaml-version in src/*/.ocamlformat!
    (self: super: { ocamlPackages = self.ocaml-ng.ocamlPackages_4_14; })

    (self: super: rec {
      # Additional ocaml packages
      ocamlPackages = super.ocamlPackages // rec {

        # downgrade wasm until we have support for 2.0.1
        # (https://github.com/dfinity/motoko/pull/3364)
        wasm_1 = super.ocamlPackages.wasm.overrideAttrs {
          version = "1.1.1";
          src = self.sources.wasm-spec-src;
          patchPhase = ''
            substituteInPlace ./interpreter/Makefile \
              --replace-fail "+a-4-27-42-44-45" "+a-4-27-42-44-45-70"
          '';
        };

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

        # js_of_ocaml-compiler version 6.0.1 is misbehaving
        js_of_ocaml-compiler = super.ocamlPackages.js_of_ocaml-compiler.override { version = "5.9.1"; };
        js_of_ocaml = super.ocamlPackages.js_of_ocaml.override { inherit js_of_ocaml-compiler; };
        gen_js_api = super.ocamlPackages.gen_js_api.override {
          inherit js_of_ocaml-compiler;
          ojs = super.ocamlPackages.ojs.override { inherit js_of_ocaml-compiler; };
        };
        js_of_ocaml-ppx = super.ocamlPackages.js_of_ocaml-ppx.override { inherit js_of_ocaml; };
      };
    }
    )

    # Rust Nightly & Stable
    rust-overlay.overlays.default
    (self: super: {
      # When you change the rust-nightly version,
      # make sure to change the rustStdDepsHash in ./rts.nix accordingly.
      rust-nightly = self.rust-bin.nightly."2025-07-19".default.override {
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
