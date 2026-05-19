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

  # Python harness — builds AE Object Specifiers via aem's reference
  # builder, packs each through `AEM_packself`, dumps the flattened
  # bytes as hex. Each query in the catalogue prints one line
  # `<name>=<hex>` so the bench's `Blob` literals can be regenerated
  # by re-running the encoder. The harness lives here in nix — there
  # is no Python file checked into the motoko source tree.
  harness = pkgs.writeText "ae-fixtures.py" ''
    """AE compact-binary fixture generator for the motoko bench.

    Each entry in `QUERIES` corresponds to a positive test/example in
    `test/bench/object-spec.mo`. `nix run .#ae-encoder` writes each
    query's flattened AE descriptor to stdout as `<name>=<hex>`.
    """
    import sys
    from aem import app, its, Codecs, ae

    # 4-char OSType abbreviations for the bench's class/property names.
    # Stable codes; the motoko bench uses the same set so round-trip
    # decoding is meaningful.
    CL_CLIENT = 'clnt'
    PR_COUNTRY = 'cntr'
    PR_AGE = 'age '
    PR_INCOME = 'inco'

    def german_midlife_client_income():
        """every client whose country == "Germany" and 45 <= age <= 55,
        then their `.yearlyIncome` property."""
        clients = app.elements(CL_CLIENT).byfilter(
            its.property(PR_COUNTRY).eq('Germany')
              .AND(its.property(PR_AGE).ge(45))
              .AND(its.property(PR_AGE).le(55))
        )
        return clients.property(PR_INCOME)

    def every_card():
        """every card — formAbsolutePosition + enum 'all ' from root."""
        return app.elements('card')

    def count_cards():
        """count cards — `count of every card`; AE property 'pcnt' (kAECount)
        on the every-cards collection."""
        return app.elements('card').property('pcnt')

    def cards_validity_02_27():
        """every card whose validity = "02/27" — 2 matches in the bench data
        (Marie Martin i=1 j=0 and Marie Roux i=61 j=0, both yy=27 mm=2)."""
        return app.elements('card').byfilter(
            its.property('vali').eq('02/27')
        )

    def first_invalid_card():
        """first card that is not valid — formAbsolutePosition + 'fst '
        applied to the (vald == false) filter."""
        return app.elements('card').byfilter(
            its.property('vald').eq(False)
        ).first

    QUERIES = {
        'german_midlife_client_income': german_midlife_client_income,
        'every_card': every_card,
        'count_cards': count_cards,
        'cards_validity_02_27': cards_validity_02_27,
        'first_invalid_card': first_invalid_card,
    }

    codecs = Codecs()
    for name, build in QUERIES.items():
        spec = build()
        desc = spec.AEM_packself(codecs)
        print(f"{name}={desc.flatten().hex()}")
  '';
in
pkgs.writeShellApplication {
  name = "ae-encoder";
  runtimeInputs = [ (python.withPackages (_: [ appscript ])) ];
  text = ''
    exec python3 ${harness} "$@"
  '';
}
