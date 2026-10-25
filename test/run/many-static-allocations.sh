#!/usr/bin/env bash
# A canister with more static allocations than fit into one Wasm function under the
# IC's function complexity limit (1,000,000, about 20,000 calls) must still install,
# and each static value must be initialized at its own index.

# The literals are spread over small functions (G per function) so that only the
# root-array initialization grows with N; a single [Text] literal of N elements would
# itself compile to one allocation function over the limit.
N=22000
G=100
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

{
  echo 'import Prim "mo:⛔";'
  echo 'actor {'
  for ((g = 0; g < N / G; g++)); do
    echo "  func group$g(i : Nat) : Text {"
    echo '    switch i {'
    for ((i = g * G; i < (g + 1) * G; i++)); do echo "      case $i \"$i\";"; done
    echo '      case _ ""'
    echo '    }'
    echo '  };'
  done
  echo '  func literal(i : Nat) : Text {'
  echo '    switch (i / '$G') {'
  for ((g = 0; g < N / G; g++)); do echo "      case $g group$g(i);"; done
  echo '      case _ ""'
  echo '    }'
  echo '  };'
  cat <<EOF
  func decimal(n : Nat) : Text {
    let digit = Prim.charToText(Prim.nat32ToChar(Prim.natToNat32(48 + n % 10)));
    if (n < 10) digit else decimal(n / 10) # digit
  };
  public query func check() : async Bool {
    var i = 0;
    while (i < $N) {
      if (literal(i) != decimal(i)) return false;
      i += 1
    };
    true
  };
}
EOF
} > "$dir/many.mo"

moc "$dir/many.mo" -o "$dir/many.wasm" || exit 1

expected="ingress Completed: Reply: 0x4449444c016c01b3c4b1f204680100010a00000000000000000101
ingress Completed: Reply: 0x4449444c0000
Ok: Reply: 0x4449444c00017e01"
actual=$(printf 'create\ninstall rwlgt-iiaaa-aaaaa-aaaaa-cai %s ""\nquery rwlgt-iiaaa-aaaaa-aaaaa-cai check "DIDL\\x00\\x00"\n' \
  "$dir/many.wasm" | test-runner --run 2>&1 | grep -v PocketIC)
[ "$actual" = "$expected" ] || echo "$actual"
