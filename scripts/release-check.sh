#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/release-check.sh [quick|full]

Release-candidate verification gate.
  quick  fmt, clippy, Rust core/node targeted suites, web typecheck/build (default)
  full   quick plus full cargo test workspace

This does not replace external security audit.
USAGE
}

mode="${1:-quick}"
case "$mode" in
  quick|full) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

section() { echo "== $* =="; }

cd "$ROOT"
section "cargo fmt --check"
cargo fmt --check

if [[ "${RELEASE_CHECK_SKIP_CLIPPY:-0}" == "1" ]]; then
  section "cargo clippy skipped"
else
  section "cargo clippy"
  cargo clippy --workspace --all-targets -- -D warnings
fi

section "test lm_core"
cargo test -p lm_core

section "test lm_node"
cargo test -p lm_node

section "test lm_wasm"
cargo test -p lm_wasm

if [[ "$mode" == "full" ]]; then
  section "workspace cargo test"
  cargo test
fi

section "web typecheck/build"
./scripts/dev-test.sh web

section "release-check $mode ok"
