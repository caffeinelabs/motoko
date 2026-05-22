{ pkgs }:
let
  releaseVersion = import ./releaseVersion.nix { inherit pkgs; officialRelease = true; };
in
{
  inherit releaseVersion;

  as_tarball = platformName: drvs:
    pkgs.runCommand "motoko-${platformName}-${releaseVersion}.tar.gz"
      {
        allowedRequisites = [ ];
      } ''
      tmp=$(mktemp -d)
      ${pkgs.lib.concatMapStringsSep "\n" (drv: "cp -v ${drv}/bin/* $tmp") drvs}
      chmod 0755 $tmp/*
      tar -czf "$out" -C $tmp/ .
    '';

  as_js = name: drv:
    pkgs.runCommand "${name}-${releaseVersion}.js"
      {
        allowedRequisites = [ ];
      } ''
      cp -v ${drv}/bin/*.min.js $out
    '';
}
