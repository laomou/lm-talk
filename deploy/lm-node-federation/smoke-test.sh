#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

node_url() {
  case "$1" in
    a) printf 'http://localhost:8081' ;;
    b) printf 'http://localhost:8082' ;;
    c) printf 'http://localhost:8083' ;;
    *) echo "unknown node: $1" >&2; exit 2 ;;
  esac
}

token_for() {
  tr -d '\n' < "$ROOT/secrets/node-$1-token"
}

request() {
  local node="$1" path="$2"
  curl -fsS -H "authorization: Bearer $(token_for "$node")" "$(node_url "$node")$path"
}

post_json() {
  local node="$1" path="$2" body="$3"
  curl -fsS \
    -H "authorization: Bearer $(token_for "$node")" \
    -H 'content-type: application/json' \
    -d "$body" \
    "$(node_url "$node")$path"
}

for node in a b c; do
  echo "== health node-$node =="
  request "$node" /health >/dev/null
  request "$node" /control/stats >/dev/null
  echo "node-$node ok"
done

echo "== publish ContactCard-like DHT record on node-a =="
# This is a shape smoke for node reachability and DHT key derivation. It does not
# store a fake ContactCard because lm_node correctly requires signed ContactCard
# payloads. Full ContactCard publish/verify is covered by the Web/WASM flow.
KEY_JSON="$(request a '/dht/key?kind=contact-card&value=user1_smoke')"
echo "$KEY_JSON" | python3 -m json.tool >/dev/null
KEY="$(python3 - <<'PY' "$KEY_JSON"
import json, sys
print(json.loads(sys.argv[1])["key"])
PY
)"
if [[ ! "$KEY" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid dht key: $KEY" >&2
  exit 1
fi

echo "== snapshot sync node-a -> node-b =="
SNAPSHOT="$(request a /sync/snapshot)"
python3 -m json.tool <<<"$SNAPSHOT" >/dev/null
post_json b /sync/import "{\"snapshot\":$SNAPSHOT}" >/dev/null
request b /sync/status >/dev/null

echo "== federation smoke ok =="
