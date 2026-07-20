#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/dev-build.sh [all|node|web|node-admin]

Production builds only.
  all         build release lm_node, production web bundle, and node-admin /admin/ bundle (default)
  node        cargo build --release -p lm_node
  web         npm run build in apps/web
  node-admin  npm run build in apps/node-admin with NODE_ADMIN_BASE=/admin/
USAGE
}

target="${1:-all}"
case "$target" in
  all|node|web|node-admin) ;;
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

build_node_admin() {
  cd "$ROOT/apps/node-admin"
  echo "== build node-admin production for /admin/ =="
  NODE_ADMIN_BASE="${NODE_ADMIN_BASE:-/admin/}" npm run build
}

case "$target" in
  all) build_node; build_web; build_node_admin ;;
  node) build_node ;;
  web) build_web ;;
  node-admin) build_node_admin ;;
esac
