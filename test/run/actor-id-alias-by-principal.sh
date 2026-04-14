#!/usr/bin/env bash
# Test that ic:<principal> imports resolve via --actor-id-alias matching on the
# principal (2nd arg), without requiring --actor-idl.

did=$(mktemp /tmp/actor-id-alias-by-principal-XXXXXX.did)
echo "service : {}" > "$did"

tmp=$(mktemp /tmp/actor-id-alias-by-principal-XXXXXX.mo)
echo "import IC \"ic:aaaaa-aa\"; persistent actor {}" > "$tmp"

moc --check \
    --actor-id-alias mgmt aaaaa-aa "$did" \
    "$tmp" 2>&1

rm -f "$did" "$tmp"
