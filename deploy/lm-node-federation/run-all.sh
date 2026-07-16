#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

run_step() {
  local name="$1"
  shift
  echo
  echo "===== $name ====="
  "$@"
}

if [[ "${LM_NODE_FEDERATION_SKIP_UP:-0}" != "1" ]]; then
  run_step "start federation stack" docker compose -f "$ROOT/docker-compose.yml" up -d --build
fi

run_step "basic federation smoke" "$ROOT/smoke-test.sh"
run_step "chaos federation smoke" "$ROOT/chaos-smoke.sh"
run_step "load federation smoke" "$ROOT/load-smoke.sh"

echo
echo "===== federation validation ok ====="
