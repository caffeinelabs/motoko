#!/usr/bin/env bash
set -euo pipefail

# Like nixbuild.sh but also returns (and, on Linux, copies back from
# nixbuild.net) the build output path. Use this when a later step needs the
# built artifacts on the local runner (e.g. uploading release files or reading
# benchmark results).
out="$($(dirname "${BASH_SOURCE[0]}")/nixbuild.sh "$@" --json | jq '.[0].outputs.out' -r)"

# On Linux the build happened on nixbuild.net, so copy the result locally.
if [[ "$(uname)" == "Linux" ]]; then
    nix copy --from ssh-ng://eu.nixbuild.net "$out"
fi

echo "$out"
