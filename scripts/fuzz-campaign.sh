#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGETS_CSV="${FUZZ_CAMPAIGN_TARGETS:-core_imports,node_dht_rpc,node_control_request}"
DURATION="${FUZZ_CAMPAIGN_DURATION:-3600}"
ARTIFACT_DIR="${FUZZ_CAMPAIGN_ARTIFACT_DIR:-$ROOT/fuzz-campaign-artifacts}"
REPORT_FILE="${FUZZ_CAMPAIGN_REPORT:-$ARTIFACT_DIR/fuzz-campaign-report.json}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RESULTS=()

IFS=',' read -r -a TARGETS <<< "$TARGETS_CSV"
mkdir -p "$ARTIFACT_DIR/logs" "$ARTIFACT_DIR/corpus" "$ARTIFACT_DIR/artifacts"

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

record_result() {
  local target="$1" status="$2" duration_ms="$3" log_file="$4" artifacts_dir="$5" corpus_dir="$6"
  RESULTS+=("{\"target\":$(json_escape "$target"),\"status\":$(json_escape "$status"),\"duration\":$(json_escape "$DURATION"),\"duration_ms\":$duration_ms,\"log_file\":$(json_escape "$log_file"),\"artifacts_dir\":$(json_escape "$artifacts_dir"),\"corpus_dir\":$(json_escape "$corpus_dir")}")
}

write_report() {
  local status="$1"
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
  "duration_per_target": "$DURATION",
  "targets": [$joined],
  "triage_notes": "Fill in crash triage, minimized reproducers, fixes, and accepted residual risk before using this as release evidence."
}
JSON
}

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install with: cargo install cargo-fuzz" >&2
  write_report failed
  exit 127
fi

cd "$ROOT/fuzz"
for raw_target in "${TARGETS[@]}"; do
  target="$(echo "$raw_target" | xargs)"
  [[ -z "$target" ]] && continue
  log_file="$ARTIFACT_DIR/logs/$target.log"
  target_artifacts="$ARTIFACT_DIR/artifacts/$target"
  target_corpus="$ARTIFACT_DIR/corpus/$target"
  mkdir -p "$target_artifacts" "$target_corpus"
  echo "== fuzz campaign: $target ($DURATION) =="
  start="$(date +%s%3N)"
  if cargo fuzz run "$target" -- \
      -max_total_time="$DURATION" \
      -artifact_prefix="$target_artifacts/" \
      "$target_corpus" 2>&1 | tee "$log_file"; then
    end="$(date +%s%3N)"
    record_result "$target" ok "$((end - start))" "$log_file" "$target_artifacts" "$target_corpus"
  else
    end="$(date +%s%3N)"
    record_result "$target" failed "$((end - start))" "$log_file" "$target_artifacts" "$target_corpus"
    write_report failed
    exit 1
  fi
done

write_report ok
echo "fuzz campaign report: $REPORT_FILE"
