pkgs:
let
  # Map Nix system to binary name (note: Nix uses aarch64, binaries use arm64)
  binaryName = {
    "x86_64-linux" = "pocket-ic-x86_64-linux";
    "aarch64-linux" = "pocket-ic-arm64-linux";
    "x86_64-darwin" = "pocket-ic-x86_64-darwin";
    "aarch64-darwin" = "pocket-ic-arm64-darwin";
  }.${pkgs.system} or (throw "Unsupported system: ${pkgs.system}");

  # The pocket-ic-server is a binary that we download from github/dfinity/ic/releases.
  # Since this binary is important for our CI, we need to update it manually for now
  # in a conscious way, otherwise automated updating will result in breaking the CI and
  # possibly not knowing which pocket-ic-server version is actually good to use since
  # the dfinity CI releases versions weekly and sometimes they result in breaking changes.
  # Whenever someone would like to update the pocket-ic-server, they should get the
  # needed release tag and sha256 hashes from the dfinity releases and update them here as needed.
  releaseTag = "release-2026-03-26_04-51-base";
  baseUrl = "https://github.com/dfinity/ic/releases/download/${releaseTag}";
  sha256Map = {
    "pocket-ic-x86_64-linux" = "sha256:bb6bcc267fcd74f83b3d13f2bb14071f4b7c7fc6d4d6f0f67450e50b0e96011d";
    "pocket-ic-arm64-linux" = "sha256:d3155c403a05a5aca11fb6a72696c15b7530b41ea47c4e6a414f2e73f526580d";
    "pocket-ic-x86_64-darwin" = "sha256:5b905178cf1bd28c469a3103ca712c21f5a0501e46c02ba1fe5789c41019ecaf";
    "pocket-ic-arm64-darwin" = "sha256:76dc87bda23670e30168be3effb99fc8d62f7e3f9af375c6c6c2671e53c21410";
  };

  server = pkgs.stdenv.mkDerivation rec {
    name = "pocket-ic-server";

    src = pkgs.fetchurl {
      url = "${baseUrl}/${binaryName}.gz";
      sha256 = sha256Map.${binaryName};
      name = "pocket-ic-server.gz";
    };

    dontUnpack = true;

    nativeBuildInputs = [ pkgs.gzip ]
      ++ pkgs.lib.optional pkgs.stdenv.isLinux pkgs.autoPatchelfHook;

    buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
      pkgs.stdenv.cc.cc.lib
      pkgs.zlib
      pkgs.openssl
    ];

    installPhase = ''
      mkdir -p $out/bin
      
      # Decompress the file into the final binary path
      gunzip -c $src > $out/bin/pocket-ic-server
      
      # Make it executable
      chmod +x $out/bin/pocket-ic-server
    '';
  };

in
{
  inherit server;
}
