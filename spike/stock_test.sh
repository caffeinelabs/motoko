#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export MOC_EOP_RELEASE_RTS="$(cd .. && pwd)/rts/mo-rts-eop.wasm"
../src/_build/default/exes/moc.exe -no-link --enhanced-migration migrations_v2 v2.mo -o out/stock_unlinked.wasm 2>/dev/null
if ../src/_build/default/exes/mo_ld.exe -b out/stock_unlinked.wasm -l "$MOC_EOP_RELEASE_RTS" -o out/stock_linked.wasm; then
  echo STOCK-OK
else
  echo STOCK-FAIL
fi
