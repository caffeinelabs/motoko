#!/usr/bin/env bash
# Methods with an `encoder` or a `decoder` don't speak Candid, so they are
# left out of both the generated `.did` and the `candid:service` metadata.
src=codec-methods-candid
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

echo "--- .did ---"
moc --idl -o "$out/main.did" "$src/main.mo"
cat "$out/main.did"

echo "--- candid:service ---"
moc -o "$out/main.wasm" -c "$src/main.mo"
for m in plain plainMixed encoded decoded both mixed; do
  echo "$m: $(grep -ac "$m: (" "$out/main.wasm")"
done
