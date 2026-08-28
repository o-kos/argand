#!/usr/bin/env bash
#
# Verify that a release tag agrees with the workspace version and that the
# changelog documents it, then print that changelog section.
#
#   .github/scripts/release-notes.sh v0.1.0 [CHANGELOG.md]
#
# Run it before pushing a tag: it is the same check the release workflow makes,
# and it is cheaper to fail here than after a tag is published.

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "usage: $0 <vX.Y.Z> [changelog]" >&2
    exit 2
fi

tag=$1
changelog=${2:-"$(cd "$(dirname "$0")/../.." && pwd)/CHANGELOG.md"}

[[ $tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "tag '$tag' is not a vX.Y.Z release tag"
version=${tag#v}

# cargo pkgid reports the version cargo will actually build, and the one
# 'aspec --version' prints, without parsing a manifest by hand.
pkgid=$(cargo pkgid -p argand-cli --locked)
manifest_version=${pkgid##*@}

[ "$version" = "$manifest_version" ] ||
    fail "tag '$tag' does not match workspace version '$manifest_version'"

[ -f "$changelog" ] || fail "no changelog at '$changelog'"

# The section runs from its own heading to the next level-two heading, with
# blank lines at either end trimmed off.
section=$(awk -v want="## [$version]" '
    index($0, want) == 1 { found = 1; next }
    found && /^## / { exit }
    found { body[++n] = $0 }
    END {
        first = 1
        while (first <= n && body[first] ~ /^[[:space:]]*$/) first++
        while (n >= first && body[n] ~ /^[[:space:]]*$/) n--
        for (i = first; i <= n; i++) print body[i]
    }
' "$changelog")

[ -n "$section" ] || fail "no '## [$version]' section with content in $changelog"

printf '%s\n' "$section"
