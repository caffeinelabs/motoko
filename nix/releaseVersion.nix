# The release version: the first version number in Changelog.md, unless the
# MOTOKO_RELEASE_VERSION environment variable overrides it.
#
# The override is what lets a prerelease such as 2.0.0-beta.0 be built before
# its `## x.y.z (date)` heading exists in the changelog:
#
#   MOTOKO_RELEASE_VERSION=2.0.0-beta.0 nix build --impure ...
#
# `--impure` is required: under a pure evaluation `builtins.getEnv` returns ""
# even when the variable is set, so the build silently falls back to the
# changelog instead of failing.
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
