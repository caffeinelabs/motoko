#!/usr/bin/env bash
# Test that --actor-id-alias takes precedence over --actor-idl when both are
# present and the --actor-idl directory does NOT contain the principal's .did.
# Expects silent success (--actor-id-alias wins, no conflict warning).

idldir=$(mktemp -d /tmp/actor-id-alias-principal-over-idl-XXXXXX)

did=$(mktemp /tmp/actor-id-alias-principal-over-idl-XXXXXX.did)
echo "service : {}" > "$did"

tmp=$(mktemp /tmp/actor-id-alias-principal-over-idl-XXXXXX.mo)
echo "import IC \"ic:aaaaa-aa\"; persistent actor {}" > "$tmp"

moc --check \
    --actor-idl "$idldir" \
    --actor-id-alias mgmt aaaaa-aa "$did" \
    "$tmp" 2>&1

rm -f "$did" "$tmp"
rm -rf "$idldir"
