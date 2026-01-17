{ pkgs, commonBuildInputs, rts, officialRelease }:
let
  is_static = !pkgs.stdenv.isDarwin;

  staticpkgs =
    if is_static
    then pkgs.pkgsMusl
    else pkgs;

  releaseVersion = import ./releaseVersion.nix { inherit pkgs officialRelease; };

  ocaml_exe = name: bin: rts:
    let
      profile =
        if is_static
        then "release-static"
        else "release";
      is_dyn_static =
        is_static && pkgs.stdenv.hostPlatform.system == "aarch64-linux";
    in
    staticpkgs.stdenv.mkDerivation {
      inherit name;

      allowedRequisites = pkgs.lib.optional is_static staticpkgs.musl
        ++ pkgs.lib.optional is_dyn_static staticpkgs.patchelf;

      src = ../src;

      buildInputs = commonBuildInputs staticpkgs;

      MOTOKO_RELEASE = releaseVersion;

      extraDuneOpts = "";

      # we only need to include the wasm statically when building moc, not
      # other binaries
      buildPhase = ''
        patchShebangs .
      '' + pkgs.lib.optionalString (rts != null) ''
        ./rts/gen.sh ${rts}/rts
      '' + ''
        make DUNE_OPTS="--display=short --profile ${profile} $extraDuneOpts" ${bin}
      '';

      installPhase = ''
        mkdir -p $out/bin
        cp --verbose --dereference ${bin} $out/bin
      '' + pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
        # there are references to darwin system libraries
        # in the binaries. But curiously, we can remove them
        # and the binaries still work. They are essentially static otherwise.
        remove-references-to \
          -t ${pkgs.libiconv} \
          $out/bin/*
      '' + ''
        # also, there is a reference to /nix/store/…/share/menhir/standard.mly.
        # Let's remove that, too
        remove-references-to \
          -t ${staticpkgs.ocamlPackages.menhir} \
          $out/bin/*
      '' + pkgs.lib.optionalString (!officialRelease && is_dyn_static) ''
        # these systems need a fixup to the loader interpreter
        chmod +w $out/bin/*
        patchelf --set-interpreter "${staticpkgs.musl}/lib/ld-musl-aarch64.so.1" $out/bin/*
        chmod a-w $out/bin/*
      '';

      doInstallCheck = !officialRelease;
      installCheckPhase = ''
        $out/bin/* --help > /dev/null
      '';
    };
in
rec {
  moc = ocaml_exe "moc" "moc" rts;
  mo-ld = ocaml_exe "mo-ld" "mo-ld" null;
  mo-doc = ocaml_exe "mo-doc" "mo-doc" null;
  didc = ocaml_exe "didc" "didc" null;
  deser = ocaml_exe "deser" "deser" null;
  candid-tests = ocaml_exe "candid-tests" "candid-tests" null;

  # executable built with coverage:
  coverage_bins = builtins.listToAttrs (pkgs.lib.flip map [ moc mo-ld didc deser ] (drv:
    {
      name = drv.name;
      value = drv.overrideAttrs (old: {
        name = "${old.name}-coverage";
        extraDuneOpts = "--instrument-with bisect_ppx";
        installPhase = old.installPhase + ''
          # The coverage report needs access to sources, including generated ones
          # like _build/parser.ml
          mkdir $out/src
          find -name \*.ml -print0 | xargs -0 cp -t $out/src --parents
        '';
        allowedRequisites = null;
      });
    }));
}
