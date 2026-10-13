#!/usr/bin/env bash
set -euo pipefail

# Run from the repository's Nix development shell. These tiny test plugins
# exercise the compiler SDK without depending on any production plugin repo.
fixture_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$fixture_root"

case "${1:-}" in
  "") fixture_build_motoko=true ;;
  --skip-motoko-build) fixture_build_motoko=false ;;
  *) printf 'Usage: %s [--skip-motoko-build]\n' "$0" >&2; exit 2 ;;
esac
if [[ $# -gt 1 ]]; then
  printf 'Usage: %s [--skip-motoko-build]\n' "$0" >&2
  exit 2
fi

# OCaml 5.4 includes Bigarray in the standard library. Override the pinned
# findlib metadata when it still refers to removed standalone archives.
fixture_ocaml_version=$(ocamlc -version)
IFS=. read -r fixture_ocaml_major fixture_ocaml_minor _ <<< "$fixture_ocaml_version"
if (( fixture_ocaml_major > 5 || (fixture_ocaml_major == 5 && fixture_ocaml_minor >= 4) )) &&
    [[ ! -f "$(ocamlc -where)/bigarray.cmxa" ]]; then
  fixture_compat="$fixture_root/_build/ocaml-compat"
  mkdir -p "$fixture_compat/bigarray"
  printf 'version = "%s"\ndescription = "Bigarray is in the standard library"\nrequires = ""\n' \
    "$fixture_ocaml_version" > "$fixture_compat/bigarray/META"
  export OCAMLPATH="$fixture_compat${OCAMLPATH:+:$OCAMLPATH}"
fi

if [[ "$fixture_build_motoko" = true ]]; then
  src/source_id/gen.sh
  dune build --root src --profile=release exes/moc.exe
  ln -fs _build/default/exes/moc.exe src/moc
fi
python3 test/compute-plugins/fixture_tools.py
