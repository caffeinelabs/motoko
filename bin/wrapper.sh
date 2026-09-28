#!/usr/bin/env bash

#
# This script is only used in the nix-shell, to give easy access to
# build products, and to provide an environment that mimics the “real”
# build environment of a nix-build.
#
# It should be invoked via a symlink named like one of the build products.
#

abs_root="$(realpath "$(dirname "${BASH_SOURCE[0]}")/..")"
rel_root="$(realpath --relative-to=. "$abs_root")"

declare -A real # the real location, relative to repo root
declare -A hint # the command to build this
declare -A envs # list of expected environment variables with paths to products

# Define build products here
real[moc]=src/moc
hint[moc]="make -C $rel_root/src moc"
envs[moc]="MOC_RELEASE_RTS MOC_DEBUG_RTS"
real[mo-ld]=src/mo-ld
hint[mo-ld]="make -C $rel_root/src mo-ld"
real[mo-doc]=src/mo-doc
hint[mo-doc]="make -C $rel_root/src mo-doc"
real[didc]=src/didc
hint[didc]="make -C $rel_root/src didc"
real[deser]=src/deser
hint[deser]="make -C $rel_root/src deser"
real[candid-tests]=src/candid-tests
hint[candid-tests]="make -C $rel_root/src candid-tests"

rts_hint="make -C $rel_root/rts"

real[MOC_RELEASE_RTS]=rts/mo-rts.wasm
real[MOC_DEBUG_RTS]=rts/mo-rts-debug.wasm

for var in ${envs[moc]}; do
  hint[$var]=$rts_hint
done

# This is the command we want to run
exe=$(basename "$0")

if [ "$exe" = "wrapper.sh" ]; then
  echo "$0: Please invoke me through one of the symlinks in bin/" >&2
  exit 1
fi

# Is the target known to us
if [ ! ${real[$exe]+_} ]; then
  echo "bin/wrapper.sh: I do not know how to behave like \"$exe\"" >&2
  exit 1
fi

# Does it exist?
if [ ! -e "$abs_root/${real[$exe]}" ]; then
  echo "$0: target ${real[$exe]} not built yet." >&2
  if [ ${hint[$exe]+_} ]; then
    echo "To build, run:" >&2
    echo "    ${hint[$exe]}" >&2
    exit 1
  fi
fi

# Do we have to set environment variables for this command?
for var in ${envs[$exe]}; do
  if [ ! ${real[$var]+_} ]; then
    echo "bin/wrapper.sh: I do not know about environment variable \"$var\"" >&2
    exit 1
  fi
  if [ ! -e "$abs_root/${real[$exe]}" ]; then
    echo "$0: auxillary file ${real[$var]} not built yet." >&2
    if [ ${hint[$var]+_} ]; then
      echo "To build, run:" >&2
      echo "    ${hint[$var]}" >&2
      exit 1
    fi
  fi
  declare "$var=$abs_root/${real[$var]}"
  export ${var?}
done

exec "$abs_root/${real[$exe]}" "$@"
