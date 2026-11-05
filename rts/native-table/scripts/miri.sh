#!/usr/bin/env bash
# typed_alias_miri under Miri: the crate's cursor over raw typed nodes hands out `&mut V` that stay valid
# across later cursor calls. The Miri entry of Rust EOP (a Rust library for upgrades that keep the heap in place):
# nightly-2025-06-19 with strict provenance, once under stacked borrows and once under tree borrows, each in
# its own target directory.
set -euo pipefail
cd "$(dirname "$0")/.."
MIRI=nightly-2025-06-19

echo "== Miri, stacked borrows ($(cargo +$MIRI miri --version 2>/dev/null || echo "$MIRI"))"
MIRIFLAGS=-Zmiri-strict-provenance CARGO_TARGET_DIR=target/miri-sb \
    cargo +$MIRI miri test -q -p nt-ref --test typed_alias_miri

echo "== Miri, tree borrows"
MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-tree-borrows" CARGO_TARGET_DIR=target/miri-tb \
    cargo +$MIRI miri test -q -p nt-ref --test typed_alias_miri

echo "typed_alias_miri passed under both aliasing models"
