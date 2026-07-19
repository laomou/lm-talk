#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/dev-test.sh [all|rust|fmt|e2e|web|typecheck]

Test/check commands.
  all        fmt, Rust tests, core/node e2e, web typecheck, web build, web e2e (default)
  rust       cargo test
  fmt        cargo fmt --check
  e2e        core/node Rust e2e only
  web        web typecheck, production build, Playwright e2e
  typecheck  web typecheck only
USAGE
}

target="${1:-all}"
case "$target" in
  all|rust|fmt|e2e|web|typecheck) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

section() { echo "== $* =="; }

fmt_check() {
  cd "$ROOT"
  section "cargo fmt --check"
  cargo fmt --check
}

rust_tests() {
  cd "$ROOT"
  section "cargo test"
  cargo test
}

rust_e2e() {
  cd "$ROOT"
  section "core e2e secure flow"
  cargo test -p lm_core --test e2e_secure_flow
  section "node e2e prekey/sync/mailbox/ratchet flow"
  cargo test -p lm_node --test e2e_node_flow
}

web_typecheck() {
  cd "$ROOT/apps/web"
  section "web typecheck"
  npm run typecheck
}

web_tests() {
  web_typecheck
  cd "$ROOT/apps/web"
  section "web production build"
  npm run build
  section "web e2e"
  npm run test:e2e
}

case "$target" in
  all) fmt_check; rust_tests; rust_e2e; web_tests; section "all tests ok" ;;
  rust) rust_tests ;;
  fmt) fmt_check ;;
  e2e) rust_e2e ;;
  web) web_tests ;;
  typecheck) web_typecheck ;;
esac
