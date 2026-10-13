#!/usr/bin/env python3
"""Independent registration and generator checks for nominal prepared types."""

from pathlib import Path
import subprocess
import tempfile

from fixture_tools import MOC, PACKAGES, ROOT, compile_plugin
from packaging import compile_program, plugin_load_error


PRELUDE = '''
module P = Codegen.Plugin_codegen
module C = Mo_values.Compute_plugin
module T = Mo_types.Type
module G = Codegen.InstrList
let descriptor = C.Prepared.create ~owner:"audit_owner" ~name:"AuditKey"
'''


def registration(owner, types, bindings="[]"):
    return f'''let () = P.register P.{{
  abi = 1; name = "{owner}"; runtime = None; types = {types};
  imports = []; bindings = {bindings};
}}
'''


def binding(code, result="T.blob", name="fixturePreparedAudit"):
    return f'''
module Generator = struct
  module Make (B : P.Backend) = struct
    let compile _env _specialization arguments =
      let value = G.concat arguments in
      P.{{code = ({code}); representation = Managed}}
  end
end
let binding = C.{{
  id = "{name}"; name = "{name}";
  arguments = Fixed (fun _ -> Ok [T.blob]); type_parameters = [];
  result_type = (fun _ _ -> Ok ({result}));
  evaluate = pure (fun _ _ -> function
    | [value] -> Ok value | _ -> Error "arity");
}}
let bindings = [binding, (module Generator : P.Generator)]
'''


def package(work, name, source):
    directory = work / name
    directory.mkdir()
    filename = f"{name.replace('-', '_')}.ml"
    (directory / filename).write_text(source)
    compile_plugin(directory, filename)
    return directory


def check_repl_redaction():
    marker = "prepared-private-material-must-not-be-printed"
    program = f'''import Prim "mo:⛔";
type Alias = Prim.PreparedA;
let key : Alias = Prim.preparedA("{marker}");
let optional = ?key;
let array = [key];
let variant = #key(key);
let record = {{ key }};
let erased : Any = key;
let erasedArray : [Any] = [key];
let erasedRecord : {{ value : Any }} = {{ value = key }};
let erasedOption : ?Any = ?key;
let publicBytes : Blob = "public-visible-blob-control";
'''
    result = subprocess.run([str(MOC), "--plugin", str(PACKAGES / "prepared"),
                             "-i"], input=program, cwd=ROOT, capture_output=True,
                            text=True)
    assert result.returncode == 0, result.stderr
    output = result.stdout + result.stderr
    escaped_marker = "".join(f"\\{byte:02X}" for byte in marker.encode())
    assert marker not in output, output
    assert escaped_marker.lower() not in output.lower(), output
    assert result.stdout.count("<opaque>") >= 5, result.stdout
    assert result.stdout.count("<any>") >= 4, result.stdout
    public_bytes = "".join(f"\\{byte:02X}" for byte in
                           b"public-visible-blob-control")
    assert public_bytes in result.stdout, result.stdout


def main():
    with tempfile.TemporaryDirectory(prefix="prepared-type-audit-",
                                     dir=ROOT / "_build") as temporary:
        work = Path(temporary)
        unused = work / "unused.mo"
        unused.write_text("ignore ();\n")
        used = work / "used.mo"
        used.write_text('import Prim "mo:⛔"; '
                        'ignore Prim.fixturePreparedAudit("bytes");\n')

        load_cases = [
            ("wrong-owner", PRELUDE + registration("other_owner", "[descriptor]"),
             "belongs to another plugin"),
            ("duplicate-type", PRELUDE + registration("audit_owner", "[descriptor]") +
             'let other = C.Prepared.create ~owner:"other_owner" ~name:"AuditKey"\n' +
             registration("other_owner", "[other]"),
             "duplicate prepared plugin type"),
            ("type-binding-collision", PRELUDE + binding("value", name="AuditKey") +
             registration("audit_owner", "[descriptor]", "bindings"),
             "duplicate compute plugin binding"),
            ("binding-type-collision", PRELUDE + binding("value", name="AuditKey") +
             registration("audit_binding", "[]", "bindings") +
             registration("audit_owner", "[descriptor]"),
             "duplicate prepared plugin type"),
            ("invalid-type-name", PRELUDE.replace('~name:"AuditKey"', '~name:"0bad"'),
             "invalid prepared plugin type identifier"),
            ("empty-owner", PRELUDE.replace('~owner:"audit_owner"', '~owner:""'),
             "invalid prepared plugin type identifier"),
        ]
        for name, source, error in load_cases:
            diagnostic = plugin_load_error(unused, [package(work, name, source)])
            assert error in diagnostic, diagnostic

        codegen_cases = [
            ("unregistered-wrap", PRELUDE +
             binding("B.Prepared.wrap descriptor value") +
             registration("audit_owner", "[]", "bindings"),
             "unregistered prepared plugin type"),
            ("foreign-wrap", PRELUDE + registration("audit_owner", "[descriptor]") +
             binding("B.Prepared.wrap descriptor value") +
             registration("audit_other", "[]", "bindings"),
             "belongs to another plugin"),
            ("foreign-unwrap", PRELUDE + registration("audit_owner", "[descriptor]") +
             binding("B.Prepared.unwrap descriptor (C.Prepared.typ descriptor) value") +
             registration("audit_other", "[]", "bindings"),
             "belongs to another plugin"),
            ("deferred-foreign-wrap", PRELUDE +
             registration("audit_owner", "[descriptor]") +
             binding("B.Compare.dispatch _env ~comparator_index:0 [value] "
                     "[Wasm_exts.Types.I64Type] (fun _ _ getters -> "
                     "B.Prepared.wrap descriptor (G.concat getters))") +
             registration("audit_other", "[]", "bindings"),
             "belongs to another plugin"),
            ("blob-unwrap", PRELUDE + binding("B.Prepared.unwrap descriptor T.blob value") +
             registration("audit_owner", "[descriptor]", "bindings"),
             "prepared plugin value has the wrong type"),
            ("wrong-type-unwrap", PRELUDE +
             'let other = C.Prepared.create ~owner:"audit_owner" ~name:"AuditOther"\n' +
             binding("B.Prepared.unwrap descriptor (C.Prepared.typ other) value") +
             registration("audit_owner", "[descriptor; other]", "bindings"),
             "prepared plugin value has the wrong type"),
            ("unregistered-result", PRELUDE + binding("value", "C.Prepared.typ descriptor") +
             registration("audit_owner", "[]", "bindings"),
             "requires a concrete result type"),
        ]
        for name, source, error in codegen_cases:
            compiled = package(work, name, source)
            compile_program(used, work / f"{name}.wasm", [compiled], error=error)

    check_repl_redaction()
    print("prepared type audit: 13 registration/codegen rejection cases "
          "and REPL redaction passed")


if __name__ == "__main__":
    main()
