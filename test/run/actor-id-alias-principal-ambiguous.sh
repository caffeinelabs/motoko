#!/usr/bin/env bash
# Test that two --actor-id-alias entries with different alias names but the same
# principal emit the ambiguity warning, and that compilation still succeeds.

did=$(mktemp /tmp/actor-id-alias-principal-ambiguous-XXXXXX.did)
echo "service : {}" > "$did"

tmp=$(mktemp /tmp/actor-id-alias-principal-ambiguous-XXXXXX.mo)
echo "import IC \"ic:aaaaa-aa\"; persistent actor {}" > "$tmp"

moc --check \
    --actor-id-alias mgmt1 aaaaa-aa "$did" \
    --actor-id-alias mgmt2 aaaaa-aa "$did" \
    "$tmp" 2>&1

rm -f "$did" "$tmp"
