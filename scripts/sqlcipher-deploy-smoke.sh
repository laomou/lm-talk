#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_DIR="$(mktemp -d)"
PORT="${LM_NODE_SQLCIPHER_SMOKE_PORT:-18787}"
TOKEN="$(openssl rand -base64 24)"
PASS_FILE="$TMP_DIR/state-db-passphrase"
DB_FILE="$TMP_DIR/state.sqlite3"
LOG_FILE="${LM_NODE_SQLCIPHER_SMOKE_LOG:-$TMP_DIR/lm-node.log}"
REPORT_FILE="${LM_NODE_SQLCIPHER_SMOKE_REPORT:-}"
PID=""

write_report() {
  local status="$1"
  [[ -z "$REPORT_FILE" ]] && return 0
  python3 - <<'PY' "$REPORT_FILE" "$status" "$DB_FILE" "$LOG_FILE"
import json, pathlib, sys, time
report_file, status, db_file, log_file = sys.argv[1:5]
ok = status == "ok"
report = {
    "status": status,
    "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "state_db": db_file,
    "log_file": log_file,
    "stats_state_db_encrypted": ok,
    "metrics_state_db_encrypted": ok,
    "wrong_passphrase_rejected": ok,
    "checks": [
        "serve_control_sqlcipher_state_db_metrics",
        "serve_control_sqlcipher_wrong_passphrase_rejected",
    ],
}
pathlib.Path(report_file).write_text(json.dumps(report, indent=2), encoding="utf-8")
PY
}

cleanup() {
  if [[ -n "${PID:-}" ]] && kill -0 "$PID" >/dev/null 2>&1; then
    kill "$PID" >/dev/null 2>&1 || true
    wait "$PID" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap 'write_report failed || true' ERR
trap cleanup EXIT

printf '%s\n' "$(openssl rand -base64 32)" > "$PASS_FILE"
chmod 600 "$PASS_FILE"

BIN="${LM_NODE_SQLCIPHER_BIN:-}"
if [[ -z "$BIN" ]]; then
  echo "== build SQLCipher-enabled lm_node =="
  cargo build -p lm_node --features sqlcipher
  BIN="./target/debug/lm_node"
else
  echo "== use SQLCipher-enabled lm_node: $BIN =="
fi

echo "== start serve-control with sqlcipher state_db =="
"$BIN" serve-control \
  --bind "127.0.0.1:$PORT" \
  --peer-id sqlcipher-smoke-node \
  --state-db "$DB_FILE" \
  --state-db-encryption-mode sqlcipher \
  --state-db-passphrase-file "$PASS_FILE" \
  --state-db-require-encryption true \
  --control-token "$TOKEN" \
  >"$LOG_FILE" 2>&1 &
PID="$!"

for _ in $(seq 1 50); do
  if curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$PID" >/dev/null 2>&1; then
    echo "lm_node exited early" >&2
    cat "$LOG_FILE" >&2 || true
    exit 1
  fi
  sleep 0.2
done

curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null
STATS="$(curl -fsS -H "authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/control/stats")"
METRICS="$(curl -fsS -H "authorization: Bearer $TOKEN" "http://127.0.0.1:$PORT/control/metrics")"

python3 - <<'PY' "$STATS" "$METRICS"
import json, sys
stats = json.loads(sys.argv[1])
metrics = sys.argv[2]
state_db = stats.get('state_db') or {}
assert state_db.get('encryption_mode') == 'sqlcipher', state_db
assert state_db.get('encrypted') is True, state_db
assert 'lm_node_state_db_encrypted 1' in metrics, metrics
assert 'lm_node_state_db_encryption_mode{mode="sqlcipher"} 1' in metrics, metrics
PY

# Wrong passphrase must not open the encrypted database.
WRONG_PASS_FILE="$TMP_DIR/wrong-passphrase"
printf '%s\n' wrong-passphrase > "$WRONG_PASS_FILE"
chmod 600 "$WRONG_PASS_FILE"
set +e
timeout 5s "$BIN" serve-control \
  --bind "127.0.0.1:$((PORT + 1))" \
  --peer-id sqlcipher-smoke-wrong \
  --state-db "$DB_FILE" \
  --state-db-encryption-mode sqlcipher \
  --state-db-passphrase-file "$WRONG_PASS_FILE" \
  --state-db-require-encryption true \
  --control-token "$TOKEN" \
  >"$TMP_DIR/wrong.log" 2>&1
wrong_status=$?
set -e
if [[ "$wrong_status" == "0" || "$wrong_status" == "124" ]]; then
  echo "wrong SQLCipher passphrase unexpectedly opened state_db" >&2
  cat "$TMP_DIR/wrong.log" >&2 || true
  exit 1
fi

write_report ok
echo "== sqlcipher deploy smoke ok =="
