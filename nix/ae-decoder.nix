{ pkgs, appscript-src }:

# AE compact-binary decoder for the motoko bench's canister replies.
# Mirror of `ae-encoder.nix` (same py-appscript build), reused as a
# spec-compliance probe: feed it any flattened AE descriptor (e.g. a
# hex blob from `test/bench/ok/object-spec.drun-run.ok`) and it should
# `unflatten` cleanly via Apple's own machinery (py-appscript wraps
# `AEUnflattenDescFromBytes`).
#
# Usage:
#   nix run .#ae-decoder -- 646c6532...

let
  python = pkgs.python3;

  appscript = python.pkgs.buildPythonPackage {
    pname = "appscript";
    version = "unstable";
    src = "${appscript-src}/py-appscript";
    format = "setuptools";
    propagatedBuildInputs = with python.pkgs; [ pyobjc-core pyobjc-framework-Cocoa ];
    postPatch = ''
      substituteInPlace lib/appscript/__init__.py \
        --replace-fail "__version__ = 'dev'" "__version__ = '1.3.0'"
    '';
    doCheck = false;
  };

  harness = pkgs.writeText "ae-decoder.py" ''
    """AE wire-format decoder probe.

    Reads one or more hex blobs from argv, unflattens each via
    py-appscript's AEDesc (= Apple's AEUnflattenDescFromBytes), and
    recursively prints the descriptor tree. Exits non-zero on any
    decode error.
    """
    import sys
    import binascii
    from aem import ae

    def fourcc(n):
        if isinstance(n, (bytes, bytearray)):
            return bytes(n).decode('latin-1')
        return bytes([(n >> 24) & 0xff, (n >> 16) & 0xff,
                      (n >> 8) & 0xff, n & 0xff]).decode('latin-1')

    WILD = b'****'  # typeWildCard — let py-appscript return the raw subdesc

    def dump(desc, indent=0):
        pad = "  " * indent
        t = fourcc(desc.type)
        # `list` is indexed: getitem(i, '****') returns the i'th element.
        # getitem/getparam return (typecode, AEDesc) tuples — unwrap.
        def unwrap(v):
            if isinstance(v, tuple) and len(v) == 2:
                return v[1]
            return v

        if t == 'list':
            cnt = desc.count()
            print(f"{pad}{t!r} list ({cnt} items)")
            for i in range(1, cnt + 1):
                print(f"{pad}  [{i}] ->")
                dump(unwrap(desc.getitem(i, WILD)), indent + 2)
            return
        if desc.isrecord() or t in ('obj ', 'reco', 'cmpd', 'logi'):
            print(f"{pad}{t!r} record")
            for k in (b'want', b'form', b'seld', b'from',
                      b'relo', b'obj1', b'obj2',
                      b'logc', b'term', b'kobj', b'kpos'):
                try:
                    child = unwrap(desc.getparam(k, WILD))
                except Exception:
                    continue
                print(f"{pad}  [{k.decode('latin-1')!r}] ->")
                dump(child, indent + 2)
            return
        data = desc.data
        if t == 'utxt':
            try:
                pretty = data.decode('utf-16')
            except Exception:
                pretty = data.hex()
        elif t == 'long':
            # OSType-style 32-bit ints come back native-endian.
            pretty = int.from_bytes(data, byteorder='little', signed=True)
        elif t == 'enum' or t == 'type':
            # OSType 4cc, stored big-endian on the wire, returned by
            # py-appscript as native-LE bytes — reverse to recover ASCII.
            pretty = data[::-1].decode('latin-1')
        else:
            pretty = data.hex()
        print(f"{pad}{t!r} leaf, data={pretty!r}")

    def decode_bytes(data, label=None):
        # ae.unflattendesc is the wrapper around AEUnflattenDescFromBytes.
        desc = ae.unflattendesc(data)
        header = f"=== {label} ===" if label else f"=== {len(data)} bytes ==="
        print(header)
        dump(desc)
        print()

    if len(sys.argv) < 2:
        # No args: read raw binary from stdin (pair with `xxd -r -p`).
        data = sys.stdin.buffer.read()
        if data:
            decode_bytes(data, label=f"stdin ({len(data)} bytes)")
    else:
        for i, h in enumerate(sys.argv[1:]):
            data = binascii.unhexlify(h)
            decode_bytes(data, label=f"argv#{i+1}")
  '';
in
pkgs.writeShellApplication {
  name = "ae-decoder";
  runtimeInputs = [ (python.withPackages (_: [ appscript ])) ];
  text = ''
    exec python3 ${harness} "$@"
  '';
}
