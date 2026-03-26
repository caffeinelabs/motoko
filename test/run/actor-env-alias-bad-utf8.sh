#!/usr/bin/env bash
# Test that --actor-env-alias reports an error for an invalid UTF-8 envvar name.

tmp=$(mktemp /tmp/actor-env-alias-bad-utf8-XXXXXX.mo)
echo "persistent actor {}" > "$tmp"
moc --check --actor-env-alias alias $'\x80\x81' /dev/null "$tmp" 2>&1 \
  | sed "s|$tmp|test.mo|"
rm -f "$tmp"
