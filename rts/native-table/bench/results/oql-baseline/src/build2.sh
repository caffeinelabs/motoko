#!/bin/sh
set -eu
OQL="$1"; OUT="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"
MOC="$(cd "$OQL" && mops toolchain bin moc)"
mkdir -p "$OUT"
"$MOC" --release --default-persistent-actors -W M0236,M0223,M0237,M0154 --package core "$OQL/.mops/core@2.6.0/src" --package jayson "$OQL/.mops/jayson@0.1.1/src" --package oql "$OQL/src" -o "$OUT/wh.wasm" "$HERE/wh2.mo"
"$MOC" --release --default-persistent-actors -W M0236,M0223,M0237,M0154 --package core "$OQL/.mops/core@2.6.0/src" --package oql "$OQL/src" -o "$OUT/relay.wasm" "$HERE/relay.mo"
