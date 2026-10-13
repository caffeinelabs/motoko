#!/usr/bin/env python3
"""Native package integration checks; run after this directory's build.sh."""

from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from fixture_tools import (FIXTURES, MOC, PACKAGES, ROOT, build_runtime,
                           compile_plugin)


def compile_program(source, output, packages=(), error=None):
    command = [str(MOC), "-wasi-system-api"]
    for package in packages:
        command.extend(["--plugin", str(package)])
    command.extend([str(source), "-o", str(output)])
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if error is None:
        assert result.returncode == 0, result.stderr
        return output.read_bytes()
    assert result.returncode != 0, "invalid package unexpectedly compiled"
    assert error in result.stderr, result.stderr
    assert "OOPS" not in result.stderr, result.stderr


def plugin_load_error(source, packages):
    command = [str(MOC)]
    for package in packages:
        command.extend(["--plugin", str(package)])
    command.extend(["--check", str(source)])
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    assert result.returncode == 1, (result.returncode, result.stderr)
    assert "OOPS" not in result.stderr, result.stderr
    return result.stderr


def unsupported_plugin(source, package, reason):
    diagnostic = plugin_load_error(source, [package])
    assert "unsupported compute plugin" in diagnostic, diagnostic
    assert reason in diagnostic, diagnostic
    assert "Use a plugin built for this compiler" in diagnostic, diagnostic
    assert "(moc " in diagnostic and "; OCaml " in diagnostic, diagnostic
    assert "host platform" in diagnostic, diagnostic
    return diagnostic


def check_compatibility(work, source):
    # Record an incompatible Codegen interface checksum without initializing
    # any fake native symbol from that interface.
    interface = work / "interface-mismatch"
    interface.mkdir()
    (interface / "codegen.mli").write_text(
        "type compiler_build = Incompatible_interface\n"
    )
    (interface / "interface_mismatch.ml").write_text(
        "let identity (value : Codegen.compiler_build) = value\n"
    )
    subprocess.run(["ocamlopt", "-c", "codegen.mli"], cwd=interface, check=True)
    subprocess.run(["ocamlopt", "-shared", "-o", "plugin.cmxs",
                    "interface_mismatch.ml"], cwd=interface, check=True)
    unsupported_plugin(source, interface / "plugin.cmxs",
                       "interface mismatch on Codegen")

    abi = work / "abi-mismatch"
    abi.mkdir()
    (abi / "abi_mismatch.ml").write_text(
        "module P = Codegen.Plugin_codegen\n"
        "let () = P.register P.{\n"
        "  abi = P.abi_version + 1; name = \"unsupported_abi\";\n"
        "  runtime = Some \"runtime.wasm\"; types = []; imports = []; bindings = [];\n"
        "}\n"
    )
    compile_plugin(abi, "abi_mismatch.ml")
    diagnostic = unsupported_plugin(source, abi, "requires compute plugin ABI")
    versions = re.search(r"requires compute plugin ABI (\d+), "
                         r"but this compiler supports ABI (\d+)", diagnostic)
    assert versions is not None, diagnostic
    requested, supported = map(int, versions.groups())
    assert requested == supported + 1, diagnostic

    # Initialization errors and duplicate registrations retain their own
    # classification instead of claiming a toolchain mismatch.
    initializer = work / "initializer-failure"
    initializer.mkdir()
    failure = "compatibility fixture initializer failure"
    (initializer / "initializer_failure.ml").write_text(
        f'let () = failwith "{failure}"\n'
    )
    subprocess.run(["ocamlopt", "-shared", "-o", "plugin.cmxs",
                    "initializer_failure.ml"], cwd=initializer, check=True)
    diagnostic = plugin_load_error(source, [initializer])
    assert failure in diagnostic, diagnostic
    assert "unsupported compute plugin" not in diagnostic, diagnostic
    diagnostic = plugin_load_error(source, [PACKAGES / "echo"] * 2)
    assert "duplicate compute plugin: fixture_echo" in diagnostic, diagnostic
    assert "unsupported compute plugin" not in diagnostic, diagnostic


def check_registration(work, source):
    original = (FIXTURES / "echo/fixture_echo.ml").read_text()
    duplicate = work / "duplicate-binding"
    duplicate.mkdir()
    (duplicate / "duplicate_binding.ml").write_text(original.replace(
        'name = "fixture_echo"', 'name = "fixture_duplicate"'))
    compile_plugin(duplicate, "duplicate_binding.ml")
    diagnostic = plugin_load_error(source, [PACKAGES / "echo", duplicate])
    assert "duplicate compute plugin binding: fixtureEcho" in diagnostic, diagnostic
    assert "unsupported compute plugin" not in diagnostic, diagnostic

    for field, valid in [("id", "fixture_echo"), ("name", "fixtureEcho")]:
        invalid = work / f"invalid-{field}"
        invalid.mkdir()
        (invalid / "invalid_binding.ml").write_text(original.replace(
            f'{field} = "{valid}"', f'{field} = "0invalid"'))
        compile_plugin(invalid, "invalid_binding.ml")
        diagnostic = plugin_load_error(source, [invalid])
        assert "invalid compute plugin binding identifier" in diagnostic, diagnostic


def check_codegen_only(work, source):
    invalid = work / "imports-without-runtime"
    invalid.mkdir()
    (invalid / "no_runtime.ml").write_text(
        'module P = Codegen.Plugin_codegen\n'
        'let () = P.register P.{\n'
        '  abi = 1; name = "missing_runtime"; runtime = None; types = [];\n'
        '  imports = ["missing", [], []]; bindings = [];\n'
        '}\n'
    )
    compile_plugin(invalid, "no_runtime.ml")
    diagnostic = plugin_load_error(source, [invalid])
    assert "has imports but no runtime" in diagnostic, diagnostic

    package = work / "codegen-only"
    package.mkdir()
    shutil.copyfile(PACKAGES / "callbacks/plugin.cmxs", package / "plugin.cmxs")
    used = work / "callback.mo"
    used.write_text(
        'import Prim "mo:⛔"; '
        'assert Prim.fixtureApply<Nat, Nat>('
        'func (n : Nat) : Nat { n + 1 }, 40) == 41;\n'
    )
    compile_program(used, work / "codegen-only.wasm", [package])
    for mode in [["--check"], ["-r"], ["-r", "-iR"]]:
        result = subprocess.run([str(MOC), "--plugin", str(package), *mode,
                                 str(used)], cwd=ROOT, capture_output=True,
                                text=True)
        assert result.returncode == 0, result.stderr
    assert not (package / "runtime.wasm").exists()


def check_unresolved_result(work):
    package = work / "unresolved-result"
    package.mkdir()
    original = (FIXTURES / "callbacks/fixture_callbacks.ml").read_text()
    declaration = (
        "result_type = (fun types _ -> Result.map snd (schema operation types));"
    )
    assert declaration in original
    invalid = original.replace(declaration,
                               "result_type = (fun _ _ -> Ok (T.Opt T.Pre));")
    (package / "unresolved_result.ml").write_text(invalid)
    compile_plugin(package, "unresolved_result.ml")
    source = work / "unresolved-result.mo"
    source.write_text('import Prim "mo:⛔"; '
                      'ignore Prim.fixtureIdentity<Nat>(17);\n')
    result = subprocess.run([str(MOC), "--plugin", str(package), "--check",
                             str(source)], cwd=ROOT, capture_output=True,
                            text=True)
    assert result.returncode == 1, (result.returncode, result.stderr)
    assert "[M0279]" in result.stderr, result.stderr
    assert "requires a concrete result type" in result.stderr, result.stderr
    assert "OOPS" not in result.stderr, result.stderr


def check_result_representation(work):
    original = (FIXTURES / "callbacks/fixture_callbacks.ml").read_text()
    declaration = "let managed code = P.{code; representation = Managed} in"
    assert original.count(declaration) == 1
    cases = [
        ("negative", -1, 'ignore Prim.fixtureIdentity<Nat>(17);'),
        ("single", 1, 'ignore Prim.fixtureIdentity<Nat>(17);'),
        ("scalar", 2, 'ignore Prim.fixtureIdentity<Nat>(17);'),
        ("unit-scalar", 0, 'ignore Prim.fixtureIdentity<Nat>(17);'),
        ("wrong-arity", 3,
         'type Pair = (Nat, Nat); '
         'ignore Prim.fixtureIdentity<Pair>((17, 19));'),
        ("opaque", 2,
         'func identity<T>(value : T) : T { Prim.fixtureIdentity<T>(value) }; '
         'ignore identity<(Nat, Nat)>((17, 19));'),
        ("opaque-bound", 2,
         'func identity<T <: (Nat, Nat)>(value : T) : T { '
         'Prim.fixtureIdentity<T>(value) }; '
         'ignore identity<(Nat, Nat)>((17, 19));'),
    ]
    for name, arity, program in cases:
        package = work / f"result-{name}"
        package.mkdir()
        replacement = (
            "let managed code = P.{code; "
            f"representation = Tuple ({arity})" + "} in"
        )
        (package / "invalid_representation.ml").write_text(
            original.replace(declaration, replacement)
        )
        compile_plugin(package, "invalid_representation.ml")
        source = work / f"result-{name}.mo"
        source.write_text('import Prim "mo:⛔"; ' + program + '\n')
        compile_program(source, work / f"result-{name}.wasm", [package],
                        error=("compute plugin fixture_identity returned "
                               "invalid result representation Tuple "
                               f"{arity}"))


def check_managed_mutation(work):
    # Deliberately incorrect generators must fail during compilation, before
    # invalid memory operations are emitted, with the normal codegen diagnostic.
    source = work / "managed-invalid.mo"
    source.write_text('import Prim "mo:⛔"; Prim.fixtureManagedInvalid();\n')
    record = 'T.obj T.Object ["value", T.Mut T.nat]'
    immutable = 'T.obj T.Object ["value", T.nat]'
    cases = [
        ("immutable-array", "B.Arr.set env (T.Array T.nat) T.nat z z z",
         "Arr.set requires a mutable array"),
        ("wrong-element", "B.Arr.set env (T.Array (T.Mut T.nat)) T.text z z z",
         "Arr.set value type does not match the element"),
        ("non-array", "B.Arr.set env T.text T.nat z z z",
         "Arr.set requires a mutable array"),
        ("immutable-field", f'B.Object.set env ({immutable}) "value" T.nat z z',
         "Object.set requires a mutable field"),
        ("wrong-field", f'B.Object.set env ({record}) "value" T.text z z',
         "Object.set value type does not match the field"),
        ("missing-field", f'B.Object.load env ({record}) "missing"',
         "Object has no field missing"),
        ("non-record", 'B.Object.load env T.text "value"',
         "Object requires a concrete record type"),
        ("missing-initializer", f"B.Object.lit env ({record}) []",
         "Object.lit fields do not match the record"),
        ("duplicate-initializer", f'B.Object.lit env ({record}) '
         '["value", T.nat, (fun () -> z); "value", T.nat, (fun () -> z)]',
         "Object.lit fields do not match the record"),
        ("wrong-initializer", f'B.Object.lit env ({record}) '
         '["value", T.text, (fun () -> z)]',
         "Object.lit value type does not match field value"),
        ("abstract-constructor", f'let abstract = T.Con (Mo_types.Cons.fresh '
         f'"Abstract" (T.Abs ([], {record})), []) in '
         'B.Object.lit env abstract ["value", T.nat, (fun () -> z)]',
         "Object requires a concrete record type"),
    ]
    for name, code, error in cases:
        package = work / f"managed-{name}"
        package.mkdir()
        (package / "invalid.ml").write_text('''
module P = Codegen.Plugin_codegen
module C = Mo_values.Compute_plugin
module T = Mo_types.Type
module Generator = struct
  module Make (B : P.Backend) = struct
    let compile env _ _ =
      let z = B.compile_unboxed_zero in
      ignore z;
      let code = ''' + code + ''' in
      P.{code; representation = Managed}
  end
end
let () = P.register P.{
  abi = 1; name = "managed_invalid"; runtime = None; types = []; imports = [];
  bindings = [C.{
    id = "managed_invalid"; name = "fixtureManagedInvalid";
    arguments = Fixed (fun _ -> Ok []); type_parameters = [];
    result_type = (fun _ _ -> Ok T.unit);
    evaluate = C.pure (fun _ _ _ -> Ok (Mo_values.Value.Tup []));
  }, (module Generator)];
}
''')
        compile_plugin(package, "invalid.ml")
        compile_program(source, work / f"managed-{name}.wasm", [package],
                        error="compute plugin " + error)


def main():
    with tempfile.TemporaryDirectory(prefix="motoko-compute-packages-") as work:
        work = Path(work)
        empty = work / "unused.mo"
        empty.write_text("assert true;\n")
        check_compatibility(work, empty)
        check_registration(work, empty)
        check_codegen_only(work, empty)
        check_unresolved_result(work)
        check_result_representation(work)
        check_managed_mutation(work)
        used = work / "used.mo"
        used.write_text('import Prim "mo:⛔"; '
                        'assert Prim.fixtureEcho("abc") == "abc";\n')
        plain = compile_program(empty, work / "plain.wasm")
        loaded = compile_program(empty, work / "loaded.wasm", [
            PACKAGES / "echo", PACKAGES / "typed", PACKAGES / "callbacks",
            PACKAGES / "mutation",
        ])
        assert plain == loaded, "unused plugins changed the output module"

        # Unused runtime files are never opened. Typechecking and both
        # interpreters can use a compiler component without its runtime.
        package = work / "echo"
        package.mkdir()
        shutil.copyfile(PACKAGES / "echo/plugin.cmxs", package / "plugin.cmxs")
        assert compile_program(empty, work / "missing-unused.wasm", [package]) == plain
        compile_program(used, work / "missing.wasm", [package],
                        error="compute plugin fixture_echo:")
        for mode in [["--check"], ["-r"], ["-r", "-iR"]]:
            result = subprocess.run([str(MOC), "--plugin", str(package), *mode,
                                     str(used)], cwd=ROOT, capture_output=True,
                                    text=True)
            assert result.returncode == 0, result.stderr

        # Directory and .cmxs paths resolve the same sibling runtime.
        shutil.copyfile(PACKAGES / "echo/runtime.wasm", package / "runtime.wasm")
        expected = compile_program(used, work / "directory.wasm", [package])
        actual = compile_program(used, work / "file.wasm", [package / "plugin.cmxs"])
        assert expected == actual

        (package / "runtime.wasm").write_bytes(b"not a Wasm library")
        compile_program(used, work / "malformed.wasm", [package],
                        error="invalid runtime Wasm")

        # Four memory-info fields in a subsection claiming only three bytes.
        (package / "runtime.wasm").write_bytes(
            b"\x00asm\x01\x00\x00\x00\x00\x0f\x08dylink.0"
            b"\x01\x03\x00\x00\x00\x00"
        )
        compile_program(used, work / "malformed-dylink.wasm", [package],
                        error="invalid runtime Wasm")

        shutil.copyfile(PACKAGES / "typed/runtime.wasm", package / "runtime.wasm")
        compile_program(used, work / "wrong-signature.wasm", [package],
                        error="Type mismatch when linking")

        wrong = work / "wrong-library.wat"
        wrong.write_text((FIXTURES / "echo/runtime.wat").read_text().replace(
            '(export "transform")', '(export "other")'))
        build_runtime(wrong, package / "runtime.wasm")
        compile_program(used, work / "wrong-library.wasm", [package],
                        error="unresolved compute plugin import")

    print("compute plugin packaging checks passed")


if __name__ == "__main__":
    main()
