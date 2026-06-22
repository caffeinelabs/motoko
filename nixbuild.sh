#!/usr/bin/env bash
set -e

# Build a Nix attribute. On Linux the build is offloaded to nixbuild.net via a
# remote store (https://docs.nixbuild.net/remote-builds/#using-remote-stores),
# so each derivation gets its own machine (up to 16 CPUs) instead of sharing the
# CPUs of a single GitHub runner. On other platforms (macOS) we build locally.
NIX_ARGS=()

if [[ "$(uname)" == "Linux" ]]; then
    NIX_ARGS+=(
        --print-build-logs
        --builders ""
        --max-jobs 2
        --eval-store auto
        --store ssh-ng://eu.nixbuild.net
    )
fi

exec nix build "${NIX_ARGS[@]}" "$@"
