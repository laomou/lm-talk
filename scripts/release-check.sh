#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/release-check.sh [quick|full|fuzz-smoke]

Release-candidate verification gate.
  quick  fmt, clippy, Rust core/node targeted suites, fuzz harness cargo check, web typecheck/build (default)
  full   quick plus full cargo test workspace
  fuzz-smoke  quick plus short cargo-fuzz smoke runs for every target

This does not replace long-running fuzz campaigns, network chaos/load tests, or external security audit.
USAGE
}

mode="${1:-quick}"
case "$mode" in
  quick|full|fuzz-smoke) ;;
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

section "lm_core full tests"
cargo test -p lm_core

section "lm_node library tests"
cargo test -p lm_node --lib

section "lm_node binary tests"
cargo test -p lm_node --bin lm_node

section "node e2e tests"
cargo test -p lm_node --test e2e_node_flow
cargo test -p lm_node --test e2e_http_control_flow

section "fuzz harness compile check"
cargo check --manifest-path fuzz/Cargo.toml --bins --locked

if [[ "$mode" == "full" ]]; then
  section "workspace cargo test"
  cargo test
fi

if [[ "$mode" == "fuzz-smoke" ]]; then
  section "fuzz smoke runs"
  ./scripts/fuzz-smoke.sh
fi

section "web typecheck/build"
./scripts/dev-test.sh web

section "release-check $mode ok"
