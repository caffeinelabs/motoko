#!/usr/bin/env bash

set -e

file="$(dirname "${BASH_SOURCE[0]}")/rts.ml"

if [ -z "$1" ]; then
  echo "Usage: $0 path/to/motoko/rts [--no-native-table]"
  echo
  echo "With --no-native-table, the runtimes with native tables are left out (as for moc.js)."
  echo
  echo "Note: This script is typically only invoked within a nix build."
  exit 1
fi

perl -0777 -ne 'print "let wasm_release = lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\"\n";' "$1/mo-rts.wasm" > "$file"
perl -0777 -ne 'print "let wasm_debug = lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\"\n";' "$1/mo-rts-debug.wasm" >> "$file"
if [ "$2" = "--no-native-table" ]; then
  printf 'let wasm_release_table = None\nlet wasm_debug_table = None' >> "$file"
else
  perl -0777 -ne 'print "let wasm_release_table = Some (lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\")\n";' "$1/mo-rts-table.wasm" >> "$file"
  perl -0777 -ne 'print "let wasm_debug_table = Some (lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\")";' "$1/mo-rts-table-debug.wasm" >> "$file"
fi
