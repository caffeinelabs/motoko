#!/usr/bin/env python3
"""Build small compiler SDK fixtures using the matching host's interfaces."""

from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "test/compute-plugins/fixtures"
PACKAGES = ROOT / "_build/compute-plugin-tests"
MOC = ROOT / "bin/moc"


def compiler_includes():
    includes = []
    for kind in ["byte", "native"]:
        for directory in sorted((ROOT / "src/_build/default").glob(
                f"*/.*.objs/{kind}")):
            includes.extend(["-I", str(directory)])
    return includes


def compile_plugin(directory, source):
    subprocess.run(["ocamlfind", "ocamlopt", "-package", "num,wasm",
                    *compiler_includes(), "-shared", "-o", "plugin.cmxs",
                    source], cwd=directory, check=True)


def build_runtime(source, destination):
    subprocess.run(["wat2wasm", "--enable-memory64", str(source),
                    "-o", str(destination)], check=True)
    binary = destination.read_bytes()
    # dylink.0, memory-info subsection: zero memory/table size and alignment.
    # WABT does not emit arbitrary custom sections from WAT.
    destination.write_bytes(binary[:8] +
                            b"\x00\x0f\x08dylink.0\x01\x04\x00\x00\x00\x00" +
                            binary[8:])
    subprocess.run(["wasm-validate", "--enable-memory64", str(destination)],
                   check=True)


def main():
    for name in ["echo", "typed", "callbacks", "mutation", "comparators", "prepared"]:
        directory = PACKAGES / name
        directory.mkdir(parents=True, exist_ok=True)
        source = f"fixture_{name}.ml"
        shutil.copyfile(FIXTURES / name / source, directory / source)
        compile_plugin(directory, source)
        runtime = FIXTURES / name / "runtime.wat"
        if runtime.exists():
            build_runtime(runtime, directory / "runtime.wasm")
    print("compute plugin test fixtures built")


if __name__ == "__main__":
    main()
