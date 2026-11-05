#!/bin/sh
# build-textidx.sh <oql copy> <out dir>: oqltext.wasm with the copy's own moc, as `oqlbars.wasm` is built
# (--default-persistent-actors -fshared-code --max-stable-pages 1638400, no --force-gc).
set -eu
OQL="$1"; OUT="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"
MOC="$(cd "$OQL" && mops toolchain bin moc)"
"$MOC" --version
mkdir -p "$OUT"
"$MOC" --default-persistent-actors -fshared-code --max-stable-pages 1638400 -W M0236,M0223,M0237 \
  --package core "$OQL/.mops/core@2.6.0/src" --package jayson "$OQL/.mops/jayson@0.1.1/src" --package oql "$OQL/src" \
  -o "$OUT/oqltext.wasm" "$HERE/oqltext.mo"
ls -l "$OUT"
