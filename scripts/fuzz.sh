#!/usr/bin/env bash
set -euo pipefail

TARGET=${1:-core_imports}
shift || true

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install with: cargo install cargo-fuzz" >&2
  exit 127
fi

cd "$(dirname "$0")/../fuzz"
exec cargo fuzz run "$TARGET" -- "$@"
