#!/usr/bin/env bash

set -e

file="$(dirname "${BASH_SOURCE[0]}")/rts.ml"

if [ -z "$1" ]; then
  echo "Usage: $0 path/to/motoko/rts"
  echo
  echo "Note: This script is typically only invoked within a nix build."
  exit 1
fi

perl -0777 -ne 'print "let wasm_eop_release = lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\"\n";' "$1/mo-rts-eop.wasm" > "$file"
perl -0777 -ne 'print "let wasm_eop_debug = lazy \""; printf "\\x%02x", $_ for unpack("C*", $_); print "\"";' "$1/mo-rts-eop-debug.wasm" >> "$file"
