pkgs:
let
  # Map Nix system to binary name (note: Nix uses aarch64, binaries use arm64)
  binaryName = {
    "x86_64-linux" = "pocket-ic-x86_64-linux";
    "aarch64-linux" = "pocket-ic-arm64-linux";
    "x86_64-darwin" = "pocket-ic-x86_64-darwin";
    "aarch64-darwin" = "pocket-ic-arm64-darwin";
  }.${pkgs.stdenv.hostPlatform.system} or (throw "Unsupported system: ${pkgs.stdenv.hostPlatform.system}");

  # The pocket-ic-server is a binary that we download from github/dfinity/ic/releases.
  # Since this binary is important for our CI, we need to update it manually for now
  # in a conscious way, otherwise automated updating will result in breaking the CI and
  # possibly not knowing which pocket-ic-server version is actually good to use since
  # the dfinity CI releases versions weekly and sometimes they result in breaking changes.
  # Whenever someone would like to update the pocket-ic-server, they should get the
  # needed release tag and sha256 hashes from the dfinity releases and update them here as needed.
  releaseTag = "release-2026-04-30_04-26-base";
  baseUrl = "https://github.com/dfinity/ic/releases/download/${releaseTag}";
  sha256Map = {
    "pocket-ic-x86_64-linux" = "sha256:25ded0a0a7c5d1974d17f57ecec2f75dd3a6814f6ab873a3eb8e0b3c65b82bc4";
    "pocket-ic-arm64-linux" = "sha256:a05f6d52898d2ae080bc108af02f42e3e69ba5ea696f5a5a4c4ddc886760588e";
    "pocket-ic-x86_64-darwin" = "sha256:f60229b51fd4e85f95d37d61046de6f2d4cf27bffbe1751b9700741d50a58e8d";
    "pocket-ic-arm64-darwin" = "sha256:8d46c6acedd34ea2d7409666ff7d1eee266f23c0c4c523c085f1b84b5e6d173e";
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
