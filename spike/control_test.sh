#!/usr/bin/env bash
# Valid unused-merge control: base has the table/pool reserves the object was
# compiled against, but never references the mco imports.
set -euo pipefail
cd "$(dirname "$0")"
export MOC_EOP_RELEASE_RTS="$(cd .. && pwd)/rts/mo-rts-eop.wasm"
MOC=../src/_build/default/exes/moc.exe
MOLD=../src/_build/default/exes/mo_ld.exe

# match the reserves the object was built with (from out/obj.counts)
N_OBJ=$(sed -n 's/.*pool=\([0-9]*\).*/\1/p' out/obj.counts | tail -1)
T_MAIN=$(sed -n 's/.*table=\([0-9]*\).*/\1/p' out/main.counts | tail -1)
T_OBJ_ABS=$(sed -n 's/.*table=\([0-9]*\).*/\1/p' out/obj.counts | tail -1)
T_OBJ=$((T_OBJ_ABS - T_MAIN))
echo "reserves: pool_extra=$N_OBJ table_extra=$T_OBJ"

$MOC -no-link --enhanced-migration migrations_v2 \
     --spike-pool-extra "$N_OBJ" --spike-table-extra "$T_OBJ" \
     v2.mo -o out/control_unlinked.wasm 2>/dev/null
$MOLD -b out/control_unlinked.wasm -mco out/obj_20250102_000000.wasm \
      -l "$MOC_EOP_RELEASE_RTS" -o drun/mco/control_unused.wasm
echo CONTROL-BUILT
