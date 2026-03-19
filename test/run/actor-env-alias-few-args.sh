#!/usr/bin/env bash
# Test that --actor-env-alias rejects fewer than 3 arguments.

check() {
  local n="$1"; shift
  result=$(moc "$@" 2>&1 | head -1)
  echo "$result" | grep -q "option '--actor-env-alias' needs an argument" \
    || echo "$n-arg: unexpected output: $result"
}

check 0 --actor-env-alias
check 1 --actor-env-alias alias
check 2 --actor-env-alias alias MY_VAR
