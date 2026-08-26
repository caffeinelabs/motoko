#!/usr/bin/env bash
# Size measurement: each heavy prod migration compiled as a frozen object
# (moc object mode), vs an identity-migration object baseline.
# Reports the standalone object wasm size = the marginal artifact cost of one
# frozen migration, including its tree-shaken mo:core closure.
set -uo pipefail
cd "$(dirname "$0")"
MOC=../src/_build/default/exes/moc.exe
export MOC_EOP_RELEASE_RTS="$(cd .. && pwd)/rts/mo-rts-eop.wasm"
HM="${HEAVY_MIGRATIONS_DIR:?set to the private migration corpus directory}"
mkdir -p out/heavy

# identity baseline: consumes/produces one field
mkdir -p out/heavy/id
cat > out/heavy/id/lib.mo <<'EOF'
module {
  public func migration(old : { greeting : Text }) : { greeting : Text } {
    { greeting = old.greeting }
  }
}
EOF

compile_obj () { # $1 = label, $2 = migration file path
  local lab="$1" src="$2" dir
  dir=out/heavy/$lab
  mkdir -p "$dir"
  cp "$src" "$dir/mig.mo"
  cat > "$dir/wrap.mo" <<EOF
import M "mig";
let mco_entry = M.migration;
EOF
  if $MOC -no-link --spike-mco-object "$lab" \
       --spike-pool-offset 1000 --spike-table-offset 1000 --spike-segment-offset 1000 \
       --package core "$MOTOKO_CORE" \
       "$dir/wrap.mo" -o "$dir/obj.wasm" 2> "$dir/err.txt"; then
    local sz srcsz
    sz=$(wc -c < "$dir/obj.wasm")
    srcsz=$(wc -c < "$src")
    printf "%-28s src=%7d  object=%8d bytes\n" "$lab" "$srcsz" "$sz"
  else
    printf "%-28s FAILED: %s\n" "$lab" "$(grep -m1 -oE 'M[0-9]{4}.*|error.*' "$dir/err.txt" | head -c 100)"
  fi
}

compile_obj identity out/heavy/id/lib.mo
for d in "$HM"/*/; do
  name=$(basename "$d")
  f=$(ls "$d"/*.mo 2>/dev/null | head -1)
  [ -n "$f" ] && compile_obj "$name" "$f"
done
