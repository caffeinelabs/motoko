#!/usr/bin/env python3
"""Positive codegen evidence and bounded specialization; run after build.sh."""

from pathlib import Path
import subprocess
import tempfile

from fixture_tools import MOC, PACKAGES, ROOT


PRELUDE = '''import Prim "mo:⛔";
type Order = { #less; #equal; #greater };
func invoke<T>(compare : (T, T) -> Order, a : T, b : T) : Int {
  Prim.fixtureAutoCompare<T>(compare, a, b)
};
'''
SUPPORTED = ["Nat", "Int", "Nat8", "Nat16", "Nat32", "Nat64",
             "Int8", "Int16", "Int32", "Int64", "Bool", "Char",
             "Text", "Blob", "Principal"]


def compile_case(directory, name, source, primitives, prelude=PRELUDE):
    program = directory / f"{name}.mo"
    output = directory / f"{name}.wasm"
    program.write_text(prelude + source)
    subprocess.run([str(MOC), "-wasi-system-api", "--plugin",
                    str(PACKAGES / "comparators"), str(program),
                    "-o", str(output)], cwd=ROOT, check=True,
                   capture_output=True)
    binary = output.read_bytes()
    expected = {f"fixture_scalar_{primitive}".encode()
                for primitive in primitives}
    # Name entries are length prefixed, so a delimiter avoids Nat matching
    # Nat64. Every helper name has a one-byte LEB length in these fixtures.
    for primitive in SUPPORTED:
        helper = f"fixture_scalar_{primitive}".encode()
        assert (bytes([len(helper)]) + helper in binary) == (helper in expected), (
            name, primitive, primitives)
    assert (b"plugin_comparator_kind" in binary) == bool(primitives), name
    print(f"{name}: {len(binary)} bytes; scalar arms: {primitives}")


def main():
    with tempfile.TemporaryDirectory(prefix="moc-comparator-codegen-") as temp:
        directory = Path(temp)
        compile_case(directory, "unknown-only", '''
var calls = 0;
func compare(a : Nat64, b : Nat64) : Order {
  calls += 1;
  if (a < b) #less else if (a == b) #equal else #greater
};
assert invoke<Nat64>(compare, 1, 2) == -1;
assert calls == 1;
''', [])
        compile_case(directory, "all-scalars",
                     (ROOT / "test/compute-plugins/run/comparator-scalars.mo")
                     .read_text(), SUPPORTED, prelude="")
        compile_case(directory, "nat64-only", '''
func completelyUnrelatedName(a : Nat64, b : Nat64) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
func unusedText(a : Text, b : Text) : Order {
  let c = Prim.textCompare(a, b);
  if (c < 0) #less else if (c == 0) #equal else #greater
};
let alias = { compare = completelyUnrelatedName };
assert invoke<Nat64>(alias.compare, 1, 2) == -1;
''', ["Nat64"])
        compile_case(directory, "text-only", '''
func compare(a : Text, b : Text) : Order {
  let c = Prim.textCompare(a, b);
  if (c < 0) #less else if (c == 0) #equal else #greater
};
assert invoke<Text>(compare, "a", "b") == -1;
''', ["Text"])
        compile_case(directory, "bool-only", '''
func compare(a : Bool, b : Bool) : Order {
  if (a == b) #equal else if a #greater else #less
};
assert invoke<Bool>(compare, false, true) == -1;
''', ["Bool"])
        compile_case(directory, "float-unsupported", '''
func compare(a : Float, b : Float) : Order {
  if (a < b) #less else if (a == b) #equal else #greater
};
assert invoke<Float>(compare, 1, 2) == -1;
''', [])
        compile_case(directory, "reversed-unsupported", '''
func compare(a : Nat64, b : Nat64) : Order {
  if (b < a) #less else if (b == a) #equal else #greater
};
assert invoke<Nat64>(compare, 1, 2) == 1;
''', [])


if __name__ == "__main__":
    main()
