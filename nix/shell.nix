{ pkgs
, nix-update
, base-src
, core-src
, llvmEnv
, esm
, commonBuildInputs
, rts
, js
, docs
, check-rts-formatting
, debugMoPackages
, test-runner
}:
pkgs.mkShell {
  name = "motoko-shell";

  #
  # Since building moc, and testing it, are two different derivations in we
  # have to create a fake derivation for `nix-shell` that commons up the
  # build dependencies of the two to provide a build environment that offers
  # both, while not actually building `moc`
  #
  propagatedBuildInputs =
    let
      dont_build = with debugMoPackages;
        [ moc mo-ld didc deser candid-tests ] ++
        builtins.attrValues coverage_bins;
    in
    [ pkgs.ic-wasm ] ++
    pkgs.lib.lists.unique (builtins.filter (i: !(builtins.elem i dont_build)) (
      commonBuildInputs pkgs ++
      rts.buildInputs ++
      js.moc.buildInputs ++
      docs.buildInputs ++
      check-rts-formatting.buildInputs ++
      #builtins.concatMap (d: d.buildInputs or [ ]) (builtins.attrValues tests) ++
      [
        nix-update
        pkgs.ncurses
        pkgs.ocamlPackages.merlin
        pkgs.ocamlPackages.utop
        pkgs.ocamlformat
        pkgs.ocamlPackages.ocaml-lsp
        pkgs.fswatch
        pkgs.rlwrap # for `rlwrap moc`
        pkgs.moreutils # `chronic` for `make -C test quick`
        pkgs.wabt # `wasm-validate` for `run-test`
        pkgs.openjdk
        pkgs.difftastic
        pkgs.pocket-ic.server
        pkgs.gh # GitHub CLI
        pkgs.fzf
        pkgs.jq
        test-runner
      ]
    ));

  # Add these variables to the shell environment so that
  # test-runner can find the pocket-ic binary and library.
  POCKET_IC_BIN = "${pkgs.pocket-ic.server}/bin/pocket-ic-server";

  shellHook = llvmEnv + ''
    # We need to add the ./bin directory to PATH however `nix develop` or direnv
    # might be invoked from a subdirectory of the motoko repository. So we need
    # to find the top-level directory of the motoko repository. We do this by
    # checking for the file flake.nix in the current directory or any
    # higher-level directory:
    dir="$PWD"
    while [ "$dir" != "/" ]; do
      if [ -e "$dir/flake.nix" ]; then
        break
      fi
      dir="$(dirname "$dir")"
    done
    if [ ! -e "$dir/flake.nix" ]; then
      echo "Could not find flake.nix in the motoko repository!" 1>&2
      exit 1
    fi
    export PATH="$dir/bin:$PATH"

    # some cleanup of environment variables otherwise set by nix-shell
    # that would be confusing in interactive use
    unset XDG_DATA_DIRS
  '';
  ESM = esm;
  TOMMATHSRC = pkgs.sources.libtommath-src;
  LOCALE_ARCHIVE = pkgs.lib.optionalString pkgs.stdenv.isLinux "${pkgs.glibcLocales}/lib/locale/locale-archive";
  MOTOKO_BASE = base-src;
  MOTOKO_CORE = core-src;
  CANDID_TESTS = "${pkgs.sources.candid-src}/test";

  # allow building this as a derivation, so that hydra builds and caches
  # the dependencies of shell.
  #
  # Note that we are using propagatedBuildInputs above, not just buildInputs.
  # This means that the dependencies end up in the output path, in
  # /nix/store/13d…da6-motoko-shell/nix-support/propagated-build-inputs
  # so that after `nix-build -A shell` (or just `nix-build`) they are guaranteed
  # to be present in the local nix store (else this might just download an
  # empty build result path from the nix cache.)
  phases = [ "installPhase" "fixupPhase" ];
  installPhase = ''
    mkdir $out
  '';
  preferLocalBuild = true;
  allowSubstitutes = true;
}
