#!/usr/bin/env bash
set -euo pipefail

TEST_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${LM_NODE_FEDERATION_DEPLOY_DIR:-$(cd "$TEST_ROOT/../../../deploy/lm-node-federation" && pwd)}"
REPORT_FILE="${LM_NODE_FEDERATION_REPORT:-$TEST_ROOT/federation-report.json}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RESULTS=()

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

record_result() {
  local name="$1" status="$2" duration_ms="$3"
  RESULTS+=("{\"name\":$(json_escape "$name"),\"status\":$(json_escape "$status"),\"duration_ms\":$duration_ms}")
}

write_report() {
  local status="$1"
  local finished_at
  finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local joined=""
  local item
  for item in "${RESULTS[@]}"; do
    if [[ -n "$joined" ]]; then joined+=","; fi
    joined+="$item"
  done
  cat > "$REPORT_FILE" <<JSON
{
  "status": "$status",
  "started_at": "$STARTED_AT",
  "finished_at": "$finished_at",
  "message_count": ${MESSAGE_COUNT:-25},
  "steps": [$joined]
}
JSON
}

run_step() {
  local name="$1"
  shift
  echo
  echo "===== $name ====="
  local start end duration
  start="$(date +%s%3N)"
  if "$@"; then
    end="$(date +%s%3N)"
    duration=$((end - start))
    record_result "$name" ok "$duration"
  else
    end="$(date +%s%3N)"
    duration=$((end - start))
    record_result "$name" failed "$duration"
    write_report failed
    return 1
  fi
}

if [[ "${LM_NODE_FEDERATION_SKIP_UP:-0}" != "1" ]]; then
  run_step "start federation stack" docker compose -f "$DEPLOY_ROOT/docker-compose.yml" up -d --build
fi

run_step "basic federation smoke" "$TEST_ROOT/smoke-test.sh"
run_step "chaos federation smoke" "$TEST_ROOT/chaos-smoke.sh"
run_step "load federation smoke" "$TEST_ROOT/load-smoke.sh"

write_report ok

echo
echo "===== federation validation ok ====="
echo "report=$REPORT_FILE"
