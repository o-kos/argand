#!/usr/bin/env bash
set -euo pipefail

if (( $# < 2 )); then
    echo "Usage: $0 UBUNTU_SOURCES PACKAGE..." >&2
    exit 2
fi
sources=$(realpath "$1")
shift
if [[ ! -s "$sources" ]]; then
    echo "Ubuntu source definition is missing or empty: $sources" >&2
    exit 1
fi

workspace=$(mktemp -d)
trap 'rm -rf "$workspace"' EXIT
# APT's download user must be able to traverse the temporary list directory.
chmod 755 "$workspace"
mkdir "$workspace/sources.list.d" "$workspace/lists"
options=(
    -o "Dir::Etc::sourcelist=$sources"
    -o "Dir::Etc::sourceparts=$workspace/sources.list.d"
    -o "Dir::State::lists=$workspace/lists"
    -o 'Dir::Cache::pkgcache='
    -o 'Dir::Cache::srcpkgcache='
    -o 'APT::Update::Error-Mode=any'
)

# Use the same sources and fresh indexes for both operations. Never fall back
# to the runner's cached third-party indexes after a required-source failure.
apt-get "${options[@]}" update
apt-get "${options[@]}" install --no-install-recommends -y "$@"
