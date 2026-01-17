{ self, pkgs }:
let
  common = import ./release-files-common.nix { inherit pkgs; };
  packages = self.packages.${pkgs.stdenv.hostPlatform.system};
in
pkgs.runCommandNoCC "motoko-release-${common.releaseVersion}" { } ''
  mkdir $out
  cp ${common.as_tarball "Linux-x86_64" (with packages.release; [ mo-doc moc ])} $out/motoko-Linux-x86_64-${common.releaseVersion}.tar.gz
  cp ${common.as_js "moc" packages.js.moc} $out/moc-${common.releaseVersion}.js
  cp ${common.as_js "moc-interpreter" packages.js.moc_interpreter} $out/moc-interpreter-${common.releaseVersion}.js
  tar --exclude=.github -C ${pkgs.sources.motoko-base-src} -czvf $out/motoko-base-library.tar.gz .
  tar --exclude=.github -C ${pkgs.sources.motoko-core-src} -czvf $out/motoko-core.tar.gz .
''
