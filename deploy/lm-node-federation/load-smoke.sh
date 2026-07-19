#!/usr/bin/env bash
set -euo pipefail

TEST_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${LM_NODE_FEDERATION_DEPLOY_DIR:-$TEST_ROOT}"
COMPOSE=(docker compose -f "$DEPLOY_ROOT/docker-compose.yml")
MESSAGE_COUNT="${MESSAGE_COUNT:-25}"

node_url() {
  case "$1" in
    a) printf 'http://localhost:8081' ;;
    b) printf 'http://localhost:8082' ;;
    c) printf 'http://localhost:8083' ;;
    *) echo "unknown node: $1" >&2; exit 2 ;;
  esac
}

token_for() { tr -d '\n' < "$DEPLOY_ROOT/secrets/node-$1-token"; }

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

json_field() {
  local field="$1" json="$2"
  python3 -c 'import json,sys; print(json.loads(sys.argv[2])[sys.argv[1]])' "$field" "$json"
}

for node in a b c; do
  request "$node" /health >/dev/null
  request "$node" /control/stats >/dev/null
done

IDENTITY_JSON="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node identity --passphrase load-pass)"
SENDER_PUBLIC_KEY="$(json_field identity_public_key "$IDENTITY_JSON")"
BACKUP_TEXT="$(json_field backup_text "$IDENTITY_JSON")"
RECIPIENT_JSON="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node identity --passphrase load-recipient-pass)"
RECIPIENT_USER_ID="$(json_field user_id "$RECIPIENT_JSON")"
TMP_BACKUP="$(mktemp)"
trap 'rm -f "$TMP_BACKUP"' EXIT
printf '%s' "$BACKUP_TEXT" > "$TMP_BACKUP"

echo "== push $MESSAGE_COUNT signed mailbox messages to node-a =="
for i in $(seq 1 "$MESSAGE_COUNT"); do
  MESSAGE_TEXT="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node mailbox-message --backup-file "$TMP_BACKUP" --passphrase load-pass --to-user-id "$RECIPIENT_USER_ID" --kind other --ciphertext "load-message-$i")"
  PUSH_JSON="$(python3 -c 'import json,sys; print(json.dumps({"message_text":sys.argv[1],"from_identity_public_key":sys.argv[2]}))' "$MESSAGE_TEXT" "$SENDER_PUBLIC_KEY")"
  post_json a /mailbox/push "$PUSH_JSON" >/dev/null
done

echo "== snapshot node-a into node-c and take messages =="
SNAPSHOT="$(request a /sync/snapshot)"
python3 -m json.tool <<<"$SNAPSHOT" >/dev/null
post_json c /sync/import "{\"snapshot\":$SNAPSHOT}" >/dev/null
STATUS="$(request c "/mailbox/status?user_id=$RECIPIENT_USER_ID")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); summary=body.get("summary") or {}; pending=body.get("pending", summary.get("undelivered", 0)); assert int(pending) >= int(sys.argv[2]), body' "$STATUS" "$MESSAGE_COUNT"
TAKE="$(request c "/mailbox/take?user_id=$RECIPIENT_USER_ID&limit=$MESSAGE_COUNT")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); deliveries=body.get("deliveries") or body.get("messages") or []; assert len(deliveries) >= int(sys.argv[2]), body' "$TAKE" "$MESSAGE_COUNT"

METRICS="$(request c /control/metrics)"
grep -q 'lm_node_mailbox_push_rejections_total' <<<"$METRICS"

echo "== federation load smoke ok: $MESSAGE_COUNT messages =="
