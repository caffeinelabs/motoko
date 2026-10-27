#!/usr/bin/env bash
# `-g` emits a DWARF line table (.debug_line with its .debug_line_str);
# without `-g` there are no .debug_* sections.
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

cat > "$out/main.mo" <<'MO'
func fib(n : Nat) : Nat = if (n < 2) { n } else { fib(n - 1) + fib(n - 2) };
assert fib(10) == 55;
MO

debug_sections() {
  wasm-objdump -h "$1" | grep -o '"\.debug[^"]*"' | tr -d '"'
}

moc -g -o "$out/g.wasm" -c "$out/main.mo"
echo "--- with -g ---"
debug_sections "$out/g.wasm"

moc -o "$out/plain.wasm" -c "$out/main.mo"
echo "--- without -g ---"
debug_sections "$out/plain.wasm"
