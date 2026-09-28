# MOTOKO_RELEASE_VERSION overrides the version for pre-releases, which are
# built before their `## x.y.z (date)` heading exists in Changelog.md. This
# needs `nix build --impure`, since a pure evaluation cannot read the
# environment.
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
