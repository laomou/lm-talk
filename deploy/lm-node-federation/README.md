# LM Talk three-node federation template

This template runs three local `lm_node` instances behind Caddy reverse proxies so operators can test Mailbox + DHT snapshot sync and replication behavior before deploying public nodes.

## Ports

- node A: `http://localhost:8081`
- node B: `http://localhost:8082`
- node C: `http://localhost:8083`

Each node syncs with the other two through `sync_peers`.

## Quick start

```bash
cd deploy/lm-node-federation
mkdir -p secrets
for n in a b c; do
  openssl rand -base64 32 > "secrets/node-$n-token"
  openssl rand -base64 32 > "secrets/state-file-passphrase-$n"
done
chmod 600 secrets/*
docker compose up -d --build
```

Use a node URL plus the matching token from `secrets/node-*-token` in the Web app sync settings.

## Public deployment notes

For real public nodes:

1. Replace the `Caddyfile.*` `:80` sites with real HTTPS hostnames.
2. Update each `sync_peers[].url` to the public HTTPS URL of the peer.
3. Use unique peer IDs, unique tokens, and encrypted persistent volumes.
4. Keep `state_db_encryption_mode = external` until DB-level SQLCipher/equivalent support lands.
5. Set `cors_allow_origins` to your deployed Web origins.

This template is for federation/bootstrap testing. Run long-lived nodes with monitoring on `/health`, `/control/stats`, and `/control/metrics`.
