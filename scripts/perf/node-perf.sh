#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DRIVER_MANIFEST="$ROOT/scripts/perf/node-perf-driver/Cargo.toml"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/perf/node-perf.sh [options]

Run an lm_node control-plane load simulation. By default it builds the current
release lm_node, starts one isolated loopback node with a temporary SQLite DB,
runs the driver, and writes a report below artifacts/perf/.

Options:
  --scenario NAME        api, chat, or mixed (default: mixed)
  --messages N           Mailbox messages per sequential/concurrent phase (default: 500)
  --concurrency N        Concurrent Mailbox senders (default: 8)
  --samples N            API samples (default: 100)
  --target http://HOST:PORT
                           Measure an existing plain-HTTP lm_node instead of
                           creating an isolated local node.
  --token TOKEN          Bearer token for --target. Prefer LM_NODE_PERF_TOKEN.
  --keep                 Keep the temporary isolated node directory.
  -h, --help             Show this help.

Examples:
  ./scripts/perf/node-perf.sh
  ./scripts/perf/node-perf.sh --scenario chat --messages 2000 --concurrency 16
  LM_NODE_PERF_TOKEN='secret' ./scripts/perf/node-perf.sh \
    --target http://127.0.0.1:8787 --scenario api

The driver intentionally uses a real signed identity and mailbox payloads, so
requests exercise lm_node parsing, signature verification, mailbox/state DB
updates, and Long Poll delivery. It does not measure Web/WASM crypto, Caddy
HTTPS, or a real LAN. HTTPS targets are intentionally not accepted by this
raw-TCP Node benchmark driver.
USAGE
}

scenario="mixed"
messages="500"
concurrency="8"
samples="100"
target=""
token="${LM_NODE_PERF_TOKEN:-}"
keep=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scenario) scenario="${2:?--scenario requires NAME}"; shift 2 ;;
    --messages) messages="${2:?--messages requires N}"; shift 2 ;;
    --concurrency) concurrency="${2:?--concurrency requires N}"; shift 2 ;;
    --samples) samples="${2:?--samples requires N}"; shift 2 ;;
    --target) target="${2:?--target requires URL}"; shift 2 ;;
    --token) token="${2:?--token requires TOKEN}"; shift 2 ;;
    --keep) keep=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$scenario" in api|chat|mixed) ;; *) echo "--scenario must be api, chat, or mixed" >&2; exit 2 ;; esac
for value in "$messages" "$concurrency" "$samples"; do
  [[ "$value" =~ ^[1-9][0-9]*$ ]] || { echo "messages/concurrency/samples must be positive integers" >&2; exit 2; }
done
if [[ -n "$target" && ! "$target" =~ ^http://[^/]+/?$ ]]; then
  echo "--target must be a plain HTTP origin, e.g. http://127.0.0.1:8787" >&2
  exit 2
fi

stamp="$(date +%Y%m%d-%H%M%S)"
report_dir="$ROOT/artifacts/perf/$stamp"
mkdir -p "$report_dir"

temp_dir=""
node_pid=""
cleanup() {
  if [[ -n "$node_pid" ]]; then
    kill "$node_pid" >/dev/null 2>&1 || true
    wait "$node_pid" >/dev/null 2>&1 || true
  fi
  if [[ -n "$temp_dir" && "$keep" != "1" ]]; then
    rm -rf "$temp_dir"
  elif [[ -n "$temp_dir" ]]; then
    echo "temporary node data retained: $temp_dir"
  fi
}
trap cleanup EXIT

if [[ -z "$target" ]]; then
  temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/lm-node-perf.XXXXXX")"
  port="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  token="$(python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
)"
  target="http://127.0.0.1:$port"
  cat > "$temp_dir/node.json" <<EOF
{
  "bind": "127.0.0.1:$port",
  "peer_id": "lm-node-perf-local",
  "state_db": "$temp_dir/lm-node-state.sqlite3",
  "control_token": "$token",
  "sync_interval_seconds": 0,
  "rate_limit_window_seconds": 0,
  "rate_limit_max_requests": 0,
  "log_format": "text"
}
EOF
  echo "building current lm_node release binary"
  (cd "$ROOT" && cargo build --release -p lm_node)
  "$ROOT/target/release/lm_node" serve-control --config-file "$temp_dir/node.json" \
    >"$report_dir/node.log" 2>&1 &
  node_pid="$!"
  for _ in $(seq 1 100); do
    if curl --fail --silent "$target/api/health" >/dev/null; then break; fi
    sleep 0.05
  done
  curl --fail --silent "$target/api/health" >/dev/null || {
    echo "temporary lm_node did not become healthy; see $report_dir/node.log" >&2
    exit 1
  }
fi

echo "Node target: $target"
echo "Scenario: $scenario; messages=$messages; concurrency=$concurrency; samples=$samples"
echo "Report: $report_dir/summary.txt"

driver_args=(
  --target "$target"
  --scenario "$scenario"
  --messages "$messages"
  --concurrency "$concurrency"
  --samples "$samples"
)
if [[ -n "$token" ]]; then
  driver_args+=(--token "$token")
fi

(
  cd "$ROOT"
  cargo run --quiet --release --manifest-path "$DRIVER_MANIFEST" -- "${driver_args[@]}"
) | tee "$report_dir/summary.txt"

cat > "$report_dir/metadata.txt" <<EOF
scenario=$scenario
messages=$messages
concurrency=$concurrency
samples=$samples
target=$target
isolated_node=$([[ -n "$node_pid" ]] && echo true || echo false)
started_at=$(date -Iseconds)
EOF

echo "completed: $report_dir"
