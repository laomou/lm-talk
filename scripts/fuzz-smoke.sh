#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGETS=(core_imports node_dht_rpc node_control_request)
RUNS="${FUZZ_SMOKE_RUNS:-16}"

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install with: cargo install cargo-fuzz" >&2
  exit 127
fi

cd "$ROOT/fuzz"
for target in "${TARGETS[@]}"; do
  echo "== fuzz smoke: $target ($RUNS runs) =="
  cargo fuzz run "$target" -- -runs="$RUNS"
done
