#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"
cd "$ROOT"

echo "== cargo fmt --check =="
cargo fmt --check

echo "== cargo test =="
cargo test

echo "== e2e tests =="
./scripts/e2e.sh

echo "== web typecheck =="
cd "$ROOT/apps/web"
npm run typecheck
cd "$ROOT"

echo "== web build =="
./scripts/web-build.sh

echo "== web e2e =="
cd "$ROOT/apps/web"
npm run test:e2e
cd "$ROOT"

echo "== ok =="
