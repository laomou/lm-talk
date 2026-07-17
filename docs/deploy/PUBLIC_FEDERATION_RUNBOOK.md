# Public Federation Runbook / 公网联邦运行手册

This runbook defines how to deploy and validate a real public LM Talk federation. It is the operational procedure for producing the public-network evidence required by `docs/PRODUCTION_READINESS.md` and `docs/RELEASE_SIGNOFF.md`.

## Goal

Operate at least three public `lm_node` instances that provide:

- HTTPS control/Mailbox/DHT endpoints.
- SQLCipher-encrypted `state_db` or explicitly accepted external encrypted storage.
- Cross-node snapshot sync.
- DHT publish/find for ContactCard, PreKey, MailboxHint, and PublicPeer records.
- Mailbox push/take/ack across nodes.
- Metrics and logs suitable for release evidence.

## Minimum topology

| Node | Example domain | Role | Required capabilities |
| --- | --- | --- | --- |
| A | `node-a.example.com` | bootstrap + mailbox + DHT | `Bootstrap`, `Dht`, `Mailbox` |
| B | `node-b.example.com` | mailbox + DHT | `Dht`, `Mailbox` |
| C | `node-c.example.com` | mailbox + DHT | `Dht`, `Mailbox` |

Each node should list the other two nodes in `sync_peers`.

## Pre-deployment checklist

- [ ] DNS A/AAAA records point to each host.
- [ ] HTTPS certificates are issued and auto-renewing.
- [ ] `cors_allow_origins` contains only the intended Web origins.
- [ ] Unique `peer_id` per node.
- [ ] Unique control token per node.
- [ ] Distinct sync-peer tokens where possible.
- [ ] `state_db_encryption_mode=sqlcipher` for SQLCipher deployments, or documented external encrypted volume exception.
- [ ] `state_db_passphrase_file` exists, is not a symlink, and has `0600` permissions.
- [ ] `state_file_passphrase_file` exists and has `0600` permissions if `state_file` is configured.
- [ ] `/data` persistent volume is backed up and monitored.
- [ ] Host firewall exposes only `80/443` publicly; `8787` remains private behind reverse proxy.

## Deployment steps

1. Start from `deploy/lm-node-public` or adapt `deploy/lm-node-federation`.
2. Create secrets:

```bash
openssl rand -base64 32 > secrets/control-token
openssl rand -base64 32 > secrets/state-file-passphrase
openssl rand -base64 32 > secrets/state-db-passphrase
chmod 600 secrets/*
```

3. Configure `config.json`:

```json
{
  "bind": "0.0.0.0:8787",
  "peer_id": "node-a",
  "state_db": "/data/lm-node.sqlite3",
  "state_db_encryption_mode": "sqlcipher",
  "state_db_passphrase_file": "/run/secrets/state-db-passphrase",
  "state_db_require_encryption": true,
  "control_token_file": "/run/secrets/control-token",
  "cors_allow_origins": ["https://YOUR_WEB_ORIGIN"],
  "sync_peers": [
    { "url": "https://node-b.example.com", "peer_id": "node-b", "token_file": "/run/secrets/node-b-token" },
    { "url": "https://node-c.example.com", "peer_id": "node-c", "token_file": "/run/secrets/node-c-token" }
  ]
}
```

4. Build SQLCipher binary when required:

```bash
LM_NODE_CARGO_FEATURES=sqlcipher docker compose build --no-cache lm-node
```

5. Start:

```bash
docker compose up -d
```

## Validation commands

Set variables:

```bash
export NODE_A=https://node-a.example.com
export NODE_B=https://node-b.example.com
export NODE_C=https://node-c.example.com
export TOKEN_A=...
export TOKEN_B=...
export TOKEN_C=...
```

Health and stats:

```bash
curl -fsS "$NODE_A/health"
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/control/stats" | tee node-a-stats.json
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/control/metrics" | tee node-a-metrics.txt
```

SQLCipher evidence:

```bash
jq '.state_db.encryption_mode, .state_db.encrypted' node-a-stats.json
grep 'lm_node_state_db_encrypted 1' node-a-metrics.txt
grep 'lm_node_state_db_encryption_mode{mode="sqlcipher"} 1' node-a-metrics.txt
```

DHT maintenance:

```bash
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/dht/maintenance?factor=3&limit=8&max_targets=8" | tee node-a-dht-maintenance.json
```

Snapshot sync drill:

```bash
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/sync/snapshot" > node-a-snapshot.json
curl -fsS -H "authorization: Bearer $TOKEN_B" -H 'content-type: application/json' \
  -d "{\"snapshot\":$(cat node-a-snapshot.json)}" \
  "$NODE_B/sync/import" | tee node-b-import.json
curl -fsS -H "authorization: Bearer $TOKEN_B" "$NODE_B/sync/status" | tee node-b-sync-status.json
```

## Required release evidence

Archive the following for each public federation run:

- Sanitized `config.json` for each node.
- Reverse proxy config for each node.
- `/health` output for each node.
- `/control/stats` and `/control/metrics` for each node.
- SQLCipher encrypted state DB proof for each node using SQLCipher.
- DHT maintenance output.
- Snapshot export/import output.
- Mailbox push/take/ack drill output.
- Node outage recovery drill output.
- Load test report with message counts, duration, failures, and metrics.
- Logs for the validation window.

## Operational drills

### Node outage recovery

- Stop node B.
- Push Mailbox messages to node A.
- Restart node B.
- Import or wait for sync from node A.
- Verify node B can take messages.

### ContactCard / PreKey / MailboxHint / PublicPeer discovery

- Publish each record type from a Web client or node helper.
- Verify it can be found from at least one other node.
- Verify stale/invalid records are rejected.

### Token rotation

- Configure `control_previous_tokens` with old token.
- Deploy new token.
- Verify old token works during grace period and is later removed.

## Go / no-go for public federation evidence

A public federation run is **NO-GO** as production evidence if:

- Any node lacks HTTPS.
- Any control endpoint accepts unauthenticated non-health requests.
- SQLCipher mode is expected but metrics do not show `state_db_encrypted 1`.
- Snapshot sync fails between nodes.
- Mailbox push/take fails across nodes.
- DHT ContactCard/PreKey publish/find fails across nodes.
- Logs show repeated panics or unbounded rate-limit/quota failures.

## Report template

| Item | Artifact/link | Status | Notes |
| --- | --- | --- | --- |
| Node A stats/metrics |  |  |  |
| Node B stats/metrics |  |  |  |
| Node C stats/metrics |  |  |  |
| SQLCipher proof |  |  |  |
| DHT publish/find |  |  |  |
| Mailbox push/take/ack |  |  |  |
| Outage recovery |  |  |  |
| Load test |  |  |  |
| Logs |  |  |  |
