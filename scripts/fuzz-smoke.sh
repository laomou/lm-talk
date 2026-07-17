#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGETS=(core_imports node_dht_rpc node_control_request)
RUNS="${FUZZ_SMOKE_RUNS:-16}"
REPORT_FILE="${FUZZ_SMOKE_REPORT:-}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RESULTS=()

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

record_result() {
  local target="$1" status="$2" duration_ms="$3"
  RESULTS+=("{\"target\":$(json_escape "$target"),\"status\":$(json_escape "$status"),\"runs\":$RUNS,\"duration_ms\":$duration_ms}")
}

write_report() {
  local status="$1"
  [[ -z "$REPORT_FILE" ]] && return 0
  local finished_at joined item
  finished_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  joined=""
  for item in "${RESULTS[@]}"; do
    if [[ -n "$joined" ]]; then joined+=","; fi
    joined+="$item"
  done
  cat > "$REPORT_FILE" <<JSON
{
  "status": "$status",
  "started_at": "$STARTED_AT",
  "finished_at": "$finished_at",
  "runs_per_target": $RUNS,
  "targets": [$joined]
}
JSON
}

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install with: cargo install cargo-fuzz" >&2
  write_report failed
  exit 127
fi

cd "$ROOT/fuzz"
for target in "${TARGETS[@]}"; do
  echo "== fuzz smoke: $target ($RUNS runs) =="
  start="$(date +%s%3N)"
  if cargo fuzz run "$target" -- -runs="$RUNS"; then
    end="$(date +%s%3N)"
    record_result "$target" ok "$((end - start))"
  else
    end="$(date +%s%3N)"
    record_result "$target" failed "$((end - start))"
    write_report failed
    exit 1
  fi
done

write_report ok
