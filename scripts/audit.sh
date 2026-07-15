#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

section() { echo "== $* =="; }

cd "$ROOT"
if [[ "${SKIP_CARGO_AUDIT:-0}" != "1" ]]; then
  if ! command -v cargo-audit >/dev/null 2>&1; then
    echo "cargo-audit is required. Install with: cargo install cargo-audit" >&2
    exit 127
  fi
  section "cargo audit"
  cargo audit --deny warnings
else
  section "cargo audit skipped"
fi

section "npm audit (runtime + build toolchain)"
cd "$ROOT/apps/web"
npm audit --audit-level high
