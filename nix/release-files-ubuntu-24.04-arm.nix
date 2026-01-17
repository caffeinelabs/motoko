{ self, pkgs }:
let
  common = import ./release-files-common.nix { inherit pkgs; };
  packages = self.packages.${pkgs.stdenv.hostPlatform.system};
in
pkgs.runCommandNoCC "motoko-release-${common.releaseVersion}" { } ''
  mkdir $out
  cp ${common.as_tarball "Linux-aarch64" (with packages.release; [ mo-doc moc ])} $out/motoko-Linux-aarch64-${common.releaseVersion}.tar.gz
''
