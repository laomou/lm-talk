#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Verify an LM Talk public node.

Usage:
  deploy/lm-node-public/verify.sh --url https://node.example.com --token-file ./secrets/control-token [--out report.json]
USAGE
}

URL=""
TOKEN_FILE=""
OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --url) URL="${2:-}"; shift 2 ;;
    --token-file) TOKEN_FILE="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done
if [[ -z "$URL" || -z "$TOKEN_FILE" ]]; then
  usage >&2
  exit 2
fi
if [[ ! -f "$TOKEN_FILE" ]]; then
  echo "error: token file not found: $TOKEN_FILE" >&2
  exit 2
fi
TOKEN="$(tr -d '\r\n' < "$TOKEN_FILE")"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

health_ok=false
stats_ok=false
metrics_ok=false
auth_ok=false

if curl -fsS "$URL/health" > "$TMP_DIR/health.json"; then health_ok=true; fi
if curl -fsS -H "authorization: Bearer $TOKEN" "$URL/control/stats" > "$TMP_DIR/stats.json"; then stats_ok=true; auth_ok=true; fi
if curl -fsS -H "authorization: Bearer $TOKEN" "$URL/control/metrics" > "$TMP_DIR/metrics.txt"; then metrics_ok=true; fi

status=ok
if [[ "$health_ok" != true || "$stats_ok" != true || "$metrics_ok" != true ]]; then status=failed; fi

REPORT="$(python3 - <<'PY' "$URL" "$status" "$health_ok" "$auth_ok" "$stats_ok" "$metrics_ok"
import json, sys, time
url,status,health,auth,stats,metrics=sys.argv[1:]
print(json.dumps({
  'schema':'lm-node-public-verify-v1',
  'url':url,
  'generated_at':time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
  'status':status,
  'checks':{
    'health':health=='true',
    'auth':auth=='true',
    'stats':stats=='true',
    'metrics':metrics=='true',
  }
}, indent=2))
PY
)"

if [[ -n "$OUT" ]]; then
  printf '%s\n' "$REPORT" > "$OUT"
fi
printf '%s\n' "$REPORT"
[[ "$status" == ok ]]
