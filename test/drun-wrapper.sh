#!/usr/bin/env bash

CONFIG=$(realpath $(dirname $0)/drun.json5)

#
# This script wraps drun to
#
# * extract the methods calls from comments in the second argument
#   (typically the test source files)
# * adds the right canister ids as the destination to these calls
# * writes prometheus metrics to file descriptor 222
#   (for run.sh -p; post-processing happening in run.sh)
#


if [ -z "$1" ]
then
  echo "Usage: $0 <name>.wasm [call-script]"
  echo "or"
  echo "Usage: $0 <name>.drun"
  exit 1
fi

# check if we have a "--subnet-type application" arg
EXTRA_DRUN_ARGS=""
if [[ "$@" == *"--subnet-type application"* ]]
then
  EXTRA_DRUN_ARGS="--subnet-type application"
fi

export LANG=C.UTF-8

# on darwin, I have seen
#   thread 'MR Batch Processor' has overflowed its stack
# and this helps (default is 2MB)
export RUST_MIN_STACK=$((10*1024*1024))

# drun creates canisters with this ID:
ID=rwlgt-iiaaa-aaaaa-aaaaa-cai

# If it is an application subnet, we need to change the canister ID, otherwise pocket-ic will not work properly.
# Enable this only when running test-runner.
# For drun, we need to use the default canister ID.
if [[ "$@" == *"--subnet-type application"* ]]
then
  ID=22ajg-aqaaa-aaaap-adukq-cai
fi

# encoded `{ canister_id = $ID }`
# this is useful for `ingress aaaaa-aa start/stop_canister $PRINCIPAL`
PRINCIPAL=0x4449444c016c01b3c4b1f204680100010a00000000000000000101

if [ "${1: -5}" = ".drun" ]
then
  # work around different IDs in ic-ref-run and drun
  ( echo "create"
    LANG=C perl -npe 's,\$ID,'$ID',g; s,\$PRINCIPAL,'$PRINCIPAL',g' $1
  ) | test-runner --run -c "$CONFIG" $EXTRA_DRUN_ARGS /dev/stdin
else
  ( echo "create"
    echo "install $ID $1 0x"
    if [ -n "$2" ]
    then
      LANG=C perl -ne 'print "$1 '$ID' $2\n" if m,^//CALL (ingress|query) (.*),;print "upgrade '$ID' '"$1"' 0x\n" if m,^//CALL upgrade,; ' $2
    fi
  ) | test-runner --run -c "$CONFIG" $EXTRA_DRUN_ARGS /dev/stdin
fi
