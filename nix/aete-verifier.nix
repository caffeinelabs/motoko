{ pkgs, appscript-src }:

# AETE binary verifier — uses py-appscript's own aeteparser.py (the
# closest thing to a canonical implementation) to parse aete bytes read
# from stdin and dump the recovered terminology.
#
# Purpose: external oracle for the Rust-side fake `aete::encode` in
# icmator-agent.  If py-appscript's parser walks our bytes cleanly and
# the recovered class/property tables match the input Lingo, we know
# our wire format is correct enough to be readable by Real Tooling.
#
# Usage:
#   icmator-agent lingo | nix run .#aete-verifier
#
# Darwin-only (py-appscript needs pyobjc + Cocoa).

let
  python = pkgs.python3;

  appscript = python.pkgs.buildPythonPackage {
    pname = "appscript";
    version = "unstable";
    src = "${appscript-src}/py-appscript";
    format = "setuptools";
    propagatedBuildInputs = with python.pkgs; [ pyobjc-core pyobjc-framework-Cocoa lxml ];
    postPatch = ''
      substituteInPlace lib/appscript/__init__.py \
        --replace-fail "__version__ = 'dev'" "__version__ = '1.3.0'"
    '';
    doCheck = false;
  };

  harness = pkgs.writeText "aete-verify.py" ''
    """AETE wire-format verifier — parse stdin bytes via py-appscript."""
    import sys

    from appscript.aeteparser import Parser

    class _Aete:
        """Mock AEDesc carrying just enough surface for Parser.parse()."""
        def __init__(self, data):
            self.data = data
            # typeAETE = 'aete'; the parser accepts it (or typeAEUT 'aeut').
            self.type = b"aete"

    # Monkey-patch the parser:
    # 1. Skip the AEDesc type check (we just want the byte-level parser).
    # 2. Force big-endian integer reads.  py-appscript's `Parser.integer`
    #    uses `unpack("H", ...)` = HOST byte order, which is LE on Apple
    #    Silicon.  The canonical aete file format is BE, so we override
    #    integer to read >H explicitly.
    import appscript.aeteparser as _p
    from struct import unpack as _unpack
    def _be_integer(self):
        self._ptr += 2
        return _unpack(">H", self._data[self._ptr - 2:self._ptr])[0]
    _p.Parser.integer = _be_integer
    _orig_parse = _p.Parser.parse

    def _bytes_parse(self, aetes):
        for aete in aetes:
            self._data = aete.data
            self._ptr = 6  # major, minor, language, script — Parser starts here
            for _ in range(self.integer()):
                self.parsesuite()
        missingelements = self._foundclasscodes - self._foundelementcodes
        missingclasses = self._foundelementcodes - self._foundclasscodes
        for code in missingelements:
            self.elements.append((self._spareclassnames[code], code))
        for code in missingclasses:
            self.classes.append((self._spareclassnames[code], code))
        return (self.classes, self.enumerators, self.properties,
                self.elements, list(self.commands.values()))
    _p.Parser.parse = _bytes_parse

    raw = sys.stdin.buffer.read()
    if not raw:
        print("(empty input)", file=sys.stderr)
        sys.exit(2)

    parser = Parser()
    try:
        classes, enumerators, properties, elements, commands = parser.parse([_Aete(raw)])
    except Exception as e:
        print(f"PARSE FAILED at offset {parser._ptr}: {e}", file=sys.stderr)
        raise

    print(f"=== py-appscript parsed {len(raw)} bytes ===")
    print(f"classes ({len(classes)}):")
    for name, code in classes:
        c = code.decode("latin-1") if isinstance(code, (bytes, bytearray)) else code
        print(f"  {name!r:30}  {c!r}")
    print(f"elements ({len(elements)}):")
    for name, code in elements:
        c = code.decode("latin-1") if isinstance(code, (bytes, bytearray)) else code
        print(f"  {name!r:30}  {c!r}")
    print(f"properties ({len(properties)}):")
    for name, code in properties:
        c = code.decode("latin-1") if isinstance(code, (bytes, bytearray)) else code
        print(f"  {name!r:30}  {c!r}")
    print(f"enumerators ({len(enumerators)}):")
    for name, code in enumerators:
        c = code.decode("latin-1") if isinstance(code, (bytes, bytearray)) else code
        print(f"  {name!r:30}  {c!r}")
    print(f"commands ({len(commands)}):")
    for cmd in commands:
        print(f"  {cmd}")
  '';
in
pkgs.writeShellApplication {
  name = "aete-verifier";
  runtimeInputs = [ (python.withPackages (_: [ appscript ])) ];
  text = ''
    exec python3 ${harness} "$@"
  '';
}
