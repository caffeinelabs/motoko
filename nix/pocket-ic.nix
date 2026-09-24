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
  releaseTag = "release-2026-06-18_04-52-base";
  baseUrl = "https://github.com/dfinity/ic/releases/download/${releaseTag}";
  sha256Map = {
    "pocket-ic-x86_64-linux" = "sha256:38c5a25cb2dc99b88b2765722465b4603cfd99848785769f968b5facbb717b67";
    "pocket-ic-arm64-linux" = "sha256:66785cf289ace8fb8ca79fede48008aa16fa0aab9a3b0e45f2967655c4fc9254";
    "pocket-ic-x86_64-darwin" = "sha256:4dad5dccbd48d13383f9700df05124262ea29ace2a46e96504f9e83b554337c9";
    "pocket-ic-arm64-darwin" = "sha256:cfbcfaad1662119702566987c11abc185a7327340d12b51e3038b9e228f43c95";
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
