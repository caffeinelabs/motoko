#!/bin/sh
# Build the harness canisters against one OQL copy. $1 = OQL dir; $2 = out dir.
set -eu
OQL="$1"; OUT="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"
MOC="$(cd "$OQL" && mops toolchain bin moc)"
"$MOC" --version
mkdir -p "$OUT"
for f in wh relay; do
  "$MOC" --release --default-persistent-actors -W M0236,M0223,M0237 \
    --package core "$OQL/.mops/core@2.6.0/src" --package jayson "$OQL/.mops/jayson@0.1.1/src" --package oql "$OQL/src" \
    -o "$OUT/$f.wasm" "$HERE/$f.mo"
done
ls -l "$OUT"
