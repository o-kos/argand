#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
    echo 'usage: ci-result.sh <true|false> <linux> <windows> <macos>' >&2
    exit 2
fi

case "$1" in
    true) expected=(success success success) ;;
    false) expected=(success skipped skipped) ;;
    *) echo 'error: unknown CI mode' >&2; exit 2 ;;
esac
shift
results=("$@")
platforms=(linux windows macos)
for index in 0 1 2; do
    if [ "${results[$index]}" != "${expected[$index]}" ]; then
        echo "error: ${platforms[$index]} is ${results[$index]}, expected ${expected[$index]}" >&2
        exit 1
    fi
done

echo 'All required results for this CI mode passed.'
