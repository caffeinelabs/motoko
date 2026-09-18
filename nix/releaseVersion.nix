# Extracts the first version number in Changelog.md
#
# The env var MOTOKO_RELEASE_VERSION, when set and non-empty, overrides it.
# This is what lets a prerelease (e.g. 2.0.0-beta.0) be built before its
# `## x.y.z (date)` heading exists in the changelog:
#
#   MOTOKO_RELEASE_VERSION=2.0.0-beta.0 nix build --impure ...
#
# The `nix build` CLI needs `--impure` for this: under a pure evaluation
# `builtins.getEnv` returns "" even when the variable is set in the
# environment, so a pure evaluation falls back to the changelog. (The legacy
# `nix-instantiate` CLI reads the environment either way; all release build
# paths pass `--impure`, so the distinction does not matter here.) An unset
# variable always falls back to the changelog.
{ pkgs, officialRelease ? false }:
let
  changelogVersion =
    builtins.head (builtins.head (builtins.filter (x: x != null) (
      builtins.map (builtins.match "## ([0-9.]+).*") (
        pkgs.lib.splitString "\n" (builtins.readFile ../Changelog.md)
      )
    )));

  version =
    let
      override = builtins.getEnv "MOTOKO_RELEASE_VERSION";
    in
    if override != "" then override else changelogVersion;
in
if officialRelease then version else "${version}+"
