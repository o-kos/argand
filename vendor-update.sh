#!/bin/bash
#
# Create an optional project-local dependency tree.
#
# Usage:
#   ./vendor-update.sh sync
#   ./vendor-update.sh add <crate> [cargo add flags]
#   ./vendor-update.sh update [cargo update flags]
#
# Examples:
#   ./vendor-update.sh sync
#   ./vendor-update.sh add -p argand-dsp num-traits
#   ./vendor-update.sh update -p rustfft

set -euo pipefail

if [ ! -f "Cargo.toml" ]; then
  echo "Error: run this from the workspace root." >&2
  exit 1
fi

case "${1:-}" in
  sync)
    if [ "$#" -ne 1 ]; then
      echo "Usage: $0 sync" >&2
      exit 1
    fi
    ;;
  add|update)
    echo "--> cargo $*"
    cargo "$@"
    ;;
  *)
    echo "Usage: $0 <sync|add|update> [cargo arguments...]" >&2
    exit 1
    ;;
esac

mkdir -p .cargo

echo "--> Creating local vendored sources..."
cargo vendor --locked vendor >.cargo/vendor.toml

echo "--> Verifying offline build..."
cargo --config .cargo/vendor.toml build --locked --offline

echo
echo "Done. Commit Cargo.toml and Cargo.lock; vendor/ stays local."
