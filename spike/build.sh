#!/usr/bin/env bash
# MCO spike build driver (design/MigrationObjects.md, Phase 1).
# Produces:
#   out/ref_v1.wasm   whole-program v1 (chain [m1])            -- reference
#   out/ref_v2.wasm   whole-program v2 (chain [m1;m2])         -- reference
#   out/spike_v2.wasm v2 with m2 FROZEN as a linked object     -- under test
set -euo pipefail
cd "$(dirname "$0")"

MOC=../src/_build/default/exes/moc.exe
MOLD=../src/_build/default/exes/mo_ld.exe
RTS=$(cd .. && pwd)/rts/mo-rts-eop.wasm
LAB=20250102_000000

export MOC_EOP_RELEASE_RTS="$RTS"
export MOC_EOP_DEBUG_RTS="$(cd .. && pwd)/rts/mo-rts-eop-debug.wasm"

[ -x "$MOC" ] && [ -x "$MOLD" ] && [ -f "$RTS" ]

echo "== references (whole-program) =="
$MOC --enhanced-migration migrations_v1 v1.mo -o out/ref_v1.wasm
$MOC --enhanced-migration migrations_v2 v2.mo -o out/ref_v2.wasm

echo "== pass 1: main (unlinked, frozen $LAB), learn its counts =="
$MOC -no-link --enhanced-migration migrations_v2 --spike-mco-import $LAB \
     v2.mo -o out/main_pass1.wasm 2> out/main.counts || (cat out/main.counts; exit 1)
grep spike-counts out/main.counts
N_MAIN=$(sed -n 's/.*pool=\([0-9]*\).*/\1/p' out/main.counts | tail -1)
T_MAIN=$(sed -n 's/.*table=\([0-9]*\).*/\1/p' out/main.counts | tail -1)
S_MAIN=$(sed -n 's/.*segments=\([0-9]*\).*/\1/p' out/main.counts | tail -1)
echo "main: pool=$N_MAIN table=$T_MAIN segments=$S_MAIN"

echo "== pass 2: object at offsets =="
$MOC -no-link --spike-mco-object $LAB \
     --spike-pool-offset "$N_MAIN" --spike-table-offset "$T_MAIN" \
     --spike-segment-offset "$S_MAIN" \
     obj_wrapper.mo -o out/obj_$LAB.wasm 2> out/obj.counts || (cat out/obj.counts; exit 1)
grep spike-counts out/obj.counts
N_OBJ_ABS=$(sed -n 's/.*pool=\([0-9]*\).*/\1/p' out/obj.counts | tail -1)
T_OBJ_ABS=$(sed -n 's/.*table=\([0-9]*\).*/\1/p' out/obj.counts | tail -1)
N_OBJ=$N_OBJ_ABS                      # pool size is reported un-offset
T_OBJ=$((T_OBJ_ABS - T_MAIN))         # end_of_table starts at the offset
echo "object: pool=$N_OBJ table=$T_OBJ"

echo "== pass 3: main with reserved extras =="
$MOC -no-link --enhanced-migration migrations_v2 --spike-mco-import $LAB \
     --spike-pool-extra "$N_OBJ" --spike-table-extra "$T_OBJ" \
     v2.mo -o out/main_unlinked.wasm 2> out/main2.counts || (cat out/main2.counts; exit 1)

echo "== pass 4: merge object, link RTS =="
$MOLD -b out/main_unlinked.wasm -mco out/obj_$LAB.wasm -l "$RTS" -o out/spike_v2.wasm

ls -la out/*.wasm
echo OK
