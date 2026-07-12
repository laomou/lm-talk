#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/build.sh [all|node|web]

Production builds only.
  all   build release lm_node and production web bundle (default)
  node  cargo build --release -p lm_node
  web   npm run build in apps/web
USAGE
}

target="${1:-all}"
case "$target" in
  all|node|web) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

build_node() {
  cd "$ROOT"
  echo "== build node release =="
  cargo build --release -p lm_node
}

build_web() {
  cd "$ROOT/apps/web"
  echo "== build web production =="
  npm run build
}

case "$target" in
  all) build_node; build_web ;;
  node) build_node ;;
  web) build_web ;;
esac
