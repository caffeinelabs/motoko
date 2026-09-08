#!/usr/bin/env bash
# Under --enhanced-migration the motoko:stable-types custom section is
# stripped from the wasm, while --stable-types still emits the .most file;
# without --enhanced-migration the section is embedded as before.
src=em-strip-stable-types-section
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

moc --enhanced-orthogonal-persistence --enhanced-migration "$src/migrations" --stable-types -o "$out/em.wasm" -c "$src/main.mo"
echo "enhanced-migration stable-types sections: $(grep -ac motoko:stable-types "$out/em.wasm")"
echo "--- em.most ---"
cat "$out/em.most"

moc --enhanced-orthogonal-persistence --stable-types -o "$out/plain.wasm" -c "$src/plain.mo"
echo "plain stable-types sections: $(grep -ac motoko:stable-types "$out/plain.wasm")"
