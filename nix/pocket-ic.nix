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
  releaseTag = "release-2026-03-05_04-37-base";
  baseUrl = "https://github.com/dfinity/ic/releases/download/${releaseTag}";
  sha256Map = {
    "pocket-ic-x86_64-linux" = "sha256:26d01e590a7f3effff7a2ca8b2ea2b5137938b74aa7934db43234f141ce2db87";
    "pocket-ic-arm64-linux" = "sha256:cf4ec921f77ba91f24bcde988fb462abb7e7d00e297cfb5d47c3ccdf3f43f9d6";
    "pocket-ic-x86_64-darwin" = "sha256:7feb14b17808ef5d538c71e6f49f189409d92ead590d6a6d503c2e24b201886a";
    "pocket-ic-arm64-darwin" = "sha256:c005d294ab22f9f1b73506e03a1af3c2cf30c5328e324c8ba94b5a01e0685dc5";
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
