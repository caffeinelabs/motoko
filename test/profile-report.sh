#!/usr/bin/env bash

# This script runs the programs in perf/ with wasm-profiler instrumentation
# and dumps flamegraphs and an index.html in _profile.
#
# It expects on the path:
# moc test-runner wasm-profiler-instrument wasm-profiler-postproc flamegraph
# (Should all be present in the nix shell)

set -e

cd "$(dirname "${BASH_SOURCE[0]}")"

export LANG=C.UTF-8
# On darwin, default 2MB can trigger "thread 'MR Batch Processor' has overflowed its stack"
export RUST_MIN_STACK=$((10*1024*1024))

# drun's default canister id; used to address //CALL directives.
ID=rwlgt-iiaaa-aaaaa-aaaaa-cai

if ! [ -d perf ]; then
  echo "Did not find perf/"
  exit 1
fi

rm -rf _profile
mkdir _profile

rm -rf _profile_build
mkdir _profile_build

cat > _profile/index.html <<__END__
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Motoko prof flamegraphs</title>
  </head>
  <body>
  <h1>Motoko prof flamegraphs</h1>
__END__

for file in perf/*.mo; do
  base="$(basename "$file" .mo)"
  cat >> _profile/index.html <<__END__
  <h2>$base.mo</h1>
  <a href="$base.svg"><img src="$base.svg"></img></a>
  <a href="$base-reverse.svg"><img src="$base-reverse.svg"></img></a>
__END__
done

  cat >> _profile/index.html <<__END__
 </body>
</html>
__END__

for file in perf/*.mo; do
  base="$(basename "$file" .mo)"
  echo "Profiling $base..."
  moc --force-gc -g "$file" -o "_profile_build/$base.wasm"
  wasm-profiler-instrument --ic-system-api -i "_profile_build/$base.wasm" -o "_profile_build/$base.instrumented.wasm"

  # qr.mo takes far too long with profiling instrumentation, so limit runtime.
  # Synthesize a drun script: install the instrumented wasm and replay any
  # //CALL directives from the source file, then feed it to test-runner --run.
  wasm="_profile_build/$base.instrumented.wasm"
  {
    echo "create"
    echo "install $ID $wasm 0x"
    LANG=C perl -ne '
      print "$1 '"$ID"' $2\n" if m,^//CALL (ingress|query) (.*),;
      print "upgrade '"$ID"' '"$wasm"' 0x\n" if m,^//CALL upgrade,;
    ' "$file"
  } | timeout 20s test-runner --run |&
    wasm-profiler-postproc flamegraph "$wasm" \
    > "_profile_build/$base.flamegraph"

  flamegraph --hash --title "$base.mo" \
    < "_profile_build/$base.flamegraph" > "_profile/$base.svg"
  flamegraph --hash --title "$base.mo (reverse)" --reverse \
    < "_profile_build/$base.flamegraph" > "_profile/$base-reverse.svg"
done
