#!/usr/bin/env bash
set -euo pipefail

TEST_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${LM_NODE_FEDERATION_DEPLOY_DIR:-$TEST_ROOT}"

node_url() {
  case "$1" in
    a) printf 'http://localhost:8081' ;;
    b) printf 'http://localhost:8082' ;;
    c) printf 'http://localhost:8083' ;;
    *) echo "unknown node: $1" >&2; exit 2 ;;
  esac
}

token_for() {
  tr -d '\n' < "$DEPLOY_ROOT/secrets/node-$1-token"
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

json_field() {
  local field="$1" json="$2"
  python3 -c 'import json,sys; print(json.loads(sys.argv[2])[sys.argv[1]])' "$field" "$json"
}

for node in a b c; do
  echo "== health node-$node =="
  request "$node" /health >/dev/null
  request "$node" /control/stats >/dev/null
  echo "node-$node ok"
done

echo "== publish signed ContactCard DHT record on node-a =="
IDENTITY_JSON="$(docker compose -f "$DEPLOY_ROOT/docker-compose.yml" exec -T node-a /usr/local/bin/lm_node identity --passphrase smoke-pass)"
USER_ID="$(json_field user_id "$IDENTITY_JSON")"
BACKUP_TEXT="$(json_field backup_text "$IDENTITY_JSON")"
TMP_BACKUP="$(mktemp)"
printf '%s' "$BACKUP_TEXT" > "$TMP_BACKUP"
CONTACT_CARD="$(docker compose -f "$DEPLOY_ROOT/docker-compose.yml" exec -T node-a /usr/local/bin/lm_node contact-card --backup-file "$TMP_BACKUP" --passphrase smoke-pass --display-name Smoke)"
KEY_JSON="$(request a "/dht/key?kind=contact-card&value=$USER_ID")"
echo "$KEY_JSON" | python3 -m json.tool >/dev/null
KEY="$(json_field key "$KEY_JSON")"
NOW="$(date +%s)"
RECORD_JSON="$(python3 -c 'import json,sys; key,value,now=sys.argv[1],sys.argv[2],int(sys.argv[3]); key_bytes=list(bytes.fromhex(key)); print(json.dumps({"record":{"key":key_bytes,"kind":"ContactCard","value":value,"created_at":now,"expires_at":now+3600,"republish_at":now}}))' "$KEY" "$CONTACT_CARD" "$NOW")"
post_json a /dht/record "$RECORD_JSON" >/dev/null
FOUND="$(request a "/dht/find-value?key=$KEY&limit=8&max_peers=8&alpha=3")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); assert body.get("found"), body; assert body.get("record",{}).get("kind") == "ContactCard", body' "$FOUND"


echo "== mailbox push on node-a and take from node-b after snapshot =="
RECIPIENT_JSON="$(docker compose -f "$DEPLOY_ROOT/docker-compose.yml" exec -T node-a /usr/local/bin/lm_node identity --passphrase recipient-pass)"
RECIPIENT_USER_ID="$(json_field user_id "$RECIPIENT_JSON")"
MESSAGE_TEXT="$(docker compose -f "$DEPLOY_ROOT/docker-compose.yml" exec -T node-a /usr/local/bin/lm_node mailbox-message --backup-file "$TMP_BACKUP" --passphrase smoke-pass --to-user-id "$RECIPIENT_USER_ID" --kind other --ciphertext federation-smoke)"
rm -f "$TMP_BACKUP"
FROM_PUBLIC_KEY="$(json_field identity_public_key "$IDENTITY_JSON")"
PUSH_JSON="$(python3 -c 'import json,sys; print(json.dumps({"message_text":sys.argv[1],"from_identity_public_key":sys.argv[2]}))' "$MESSAGE_TEXT" "$FROM_PUBLIC_KEY")"
post_json a /mailbox/push "$PUSH_JSON" >/dev/null

echo "== snapshot sync node-a -> node-b =="
SNAPSHOT="$(request a /sync/snapshot)"
python3 -m json.tool <<<"$SNAPSHOT" >/dev/null
post_json b /sync/import "{\"snapshot\":$SNAPSHOT}" >/dev/null
request b /sync/status >/dev/null
FOUND_B="$(request b "/dht/find-value?key=$KEY&limit=8&max_peers=8&alpha=3")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); assert body.get("found"), body; assert body.get("record",{}).get("kind") == "ContactCard", body' "$FOUND_B"
TAKE="$(request b "/mailbox/take?user_id=$RECIPIENT_USER_ID&limit=5")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); deliveries=body.get("deliveries") or body.get("messages") or []; assert deliveries, body' "$TAKE"

echo "== federation smoke ok =="
