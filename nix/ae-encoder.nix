{ pkgs, appscript-src }:

# AE compact-binary fixture generator for `test/bench/object-spec.mo`.
# Wraps py-appscript so the bench can produce reproducible Apple Event
# Object Specifier wire-format samples without checking generated
# binaries into the motoko source tree.
#
# Darwin-only: py-appscript links against `AEvent.framework` and
# requires PyObjC; the derivation is hidden behind `lib.platforms.darwin`
# at the call site in `flake.nix`.

let
  python = pkgs.python3;

  # Build appscript as a Python package directly from the upstream
  # GitHub source. The source contains C extensions (`aem`, `appscript`)
  # that bind to AEvent.framework; nixpkgs' `buildPythonPackage` plus
  # the standard Cocoa/AppleEvents framework set is enough.
  appscript = python.pkgs.buildPythonPackage {
    pname = "appscript";
    version = "unstable";
    # The upstream repo is a meta-repo containing several subprojects;
    # the Python package lives under `py-appscript/`.
    src = "${appscript-src}/py-appscript";
    format = "setuptools";

    # PyObjC is the standard Python bridge to Apple frameworks; appscript
    # uses it to interop with AppleEvents and Cocoa. Linux builds of
    # nixpkgs don't ship pyobjc, which is one of several reasons this
    # whole derivation is darwin-only.
    propagatedBuildInputs = with python.pkgs; [ pyobjc-core pyobjc-framework-Cocoa ];

    # Upstream sets `__version__ = 'dev'` which modern setuptools (PEP 440)
    # rejects. Pin a placeholder version so the build proceeds.
    postPatch = ''
      substituteInPlace lib/appscript/__init__.py \
        --replace-fail "__version__ = 'dev'" "__version__ = '1.3.0'"
    '';

    # The repo doesn't ship a runnable test suite; skip checks.
    doCheck = false;
  };

  # Tiny Python harness. v1 just confirms that the appscript / aem
  # modules are importable; later commits grow this into a fixture
  # generator that prints `<name>=<hex>` for each pinned query in the
  # bench's catalogue. The harness source lives here, in nix — it is
  # neither AppleScript nor a Python file in the motoko source tree.
  harness = pkgs.writeText "ae-fixtures.py" ''
    """AE compact-binary fixture generator for the motoko bench.

    v1: smoke-test only — verifies the appscript/aem modules are
    importable so `nix run .#ae-encoder` exercises the whole stack.
    Future commits will accept a query name as an argv parameter and
    emit hex bytes for the matching ObjectSpec.
    """
    import sys
    import aem
    print(f"appscript/aem stack OK; aem from {aem.__file__}")
  '';
in
pkgs.writeShellApplication {
  name = "ae-encoder";
  runtimeInputs = [ (python.withPackages (_: [ appscript ])) ];
  text = ''
    exec python3 ${harness} "$@"
  '';
}
