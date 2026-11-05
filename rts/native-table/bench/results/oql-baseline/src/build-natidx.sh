#!/bin/sh
# build-natidx.sh <oql copy> <out dir>: oqlnat.wasm with the copy's own moc, as `oqlbars.wasm` is built
# (--default-persistent-actors -fshared-code --max-stable-pages 1638400, no --force-gc).
set -eu
OQL="$1"; OUT="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"
MOC="$(cd "$OQL" && mops toolchain bin moc)"
"$MOC" --version
mkdir -p "$OUT"
"$MOC" --default-persistent-actors -fshared-code --max-stable-pages 1638400 -W M0236,M0223,M0237 \
  --package core "$OQL/.mops/core@2.6.0/src" --package jayson "$OQL/.mops/jayson@0.1.1/src" --package oql "$OQL/src" \
  -o "$OUT/oqlnat.wasm" "$HERE/oqlnat.mo"
ls -l "$OUT"
