#!/usr/bin/env bash
# Resolve the release version, `<base>[-<suffix>]`.
#
# The base is the `base_version` input when set, otherwise the first version
# heading in Changelog.md. The suffix is the `version_suffix` input; its
# presence is what makes a release a prerelease.
#
# Runnable standalone for testing:
#   BASE_VERSION=2.0.0 VERSION_SUFFIX=beta.0 .github/actions/release-version/compute.sh
set -euo pipefail

BASE_VERSION="${BASE_VERSION:-}"
VERSION_SUFFIX="${VERSION_SUFFIX:-}"
CHANGELOG_PATH="${CHANGELOG_PATH:-Changelog.md}"

fail() {
  echo "::error::$1"
  exit 1
}

# The version ends up in a git tag, a release tag, and artifact filenames
# (which `mops toolchain` fetches by name), so keep it to characters that are
# safe in all three.
version_re='^[A-Za-z0-9]([A-Za-z0-9.-]*[A-Za-z0-9])?$'

base="$BASE_VERSION"
if [ -z "$base" ]; then
  [ -f "$CHANGELOG_PATH" ] || fail "no base_version given and ${CHANGELOG_PATH} not found"
  base="$(sed -nE 's/^## ([0-9]+\.[0-9]+\.[0-9]+) .*/\1/p' "$CHANGELOG_PATH" | head -n1)"
fi

# A base version is the first three components of semver: MAJOR.MINOR.PATCH.
printf '%s' "$base" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
  || fail "base version '${base}' must be MAJOR.MINOR.PATCH (e.g. 2.0.0)"

suffix="$VERSION_SUFFIX"
if [ -n "$suffix" ]; then
  printf '%s' "$suffix" | grep -Eq "$version_re" \
    || fail "version suffix '${suffix}' must start and end with a letter or digit and contain only letters, digits, dots and hyphens (e.g. beta.0, alpha-1, rc.1)"
fi

version="$base"
[ -n "$suffix" ] && version="${base}-${suffix}"

printf '%s' "$version" | grep -Eq "$version_re" \
  || fail "version '${version}' contains characters that are unsafe in a tag or filename"

emit() {
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$1=$2" >> "$GITHUB_OUTPUT"
  else
    echo "$1=$2"
  fi
}

emit version "$version"
emit base "$base"
emit suffix "$suffix"
