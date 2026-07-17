#!/usr/bin/env bash
set -euo pipefail

TEST_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_ROOT="${LM_NODE_FEDERATION_DEPLOY_DIR:-$(cd "$TEST_ROOT/../../../deploy/lm-node-federation" && pwd)}"
COMPOSE=(docker compose -f "$DEPLOY_ROOT/docker-compose.yml")

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

wait_health() {
  local node="$1"
  for _ in $(seq 1 30); do
    if request "$node" /health >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  echo "node-$node did not become healthy" >&2
  return 1
}

for node in a b c; do wait_health "$node"; done

IDENTITY_JSON="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node identity --passphrase chaos-pass)"
SENDER_PUBLIC_KEY="$(json_field identity_public_key "$IDENTITY_JSON")"
BACKUP_TEXT="$(json_field backup_text "$IDENTITY_JSON")"
RECIPIENT_JSON="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node identity --passphrase chaos-recipient-pass)"
RECIPIENT_USER_ID="$(json_field user_id "$RECIPIENT_JSON")"
TMP_BACKUP="$(mktemp)"
printf '%s' "$BACKUP_TEXT" > "$TMP_BACKUP"

cleanup() {
  rm -f "$TMP_BACKUP"
  ${COMPOSE[@]} up -d node-b caddy-b >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "== stopping node-b to simulate outage =="
${COMPOSE[@]} stop node-b caddy-b >/dev/null

for i in $(seq 1 5); do
  MESSAGE_TEXT="$(${COMPOSE[@]} exec -T node-a /usr/local/bin/lm_node mailbox-message --backup-file "$TMP_BACKUP" --passphrase chaos-pass --to-user-id "$RECIPIENT_USER_ID" --kind other --ciphertext "chaos-message-$i")"
  PUSH_JSON="$(python3 -c 'import json,sys; print(json.dumps({"message_text":sys.argv[1],"from_identity_public_key":sys.argv[2]}))' "$MESSAGE_TEXT" "$SENDER_PUBLIC_KEY")"
  post_json a /mailbox/push "$PUSH_JSON" >/dev/null
done

echo "== restarting node-b and importing node-a snapshot =="
${COMPOSE[@]} up -d node-b caddy-b >/dev/null
wait_health b
SNAPSHOT="$(request a /sync/snapshot)"
python3 -m json.tool <<<"$SNAPSHOT" >/dev/null
post_json b /sync/import "{\"snapshot\":$SNAPSHOT}" >/dev/null

TAKE="$(request b "/mailbox/take?user_id=$RECIPIENT_USER_ID&limit=10")"
python3 -c 'import json,sys; body=json.loads(sys.argv[1]); deliveries=body.get("deliveries") or body.get("messages") or []; assert len(deliveries) >= 5, body' "$TAKE"

echo "== federation chaos smoke ok =="
