# LM Talk public node deployment template

This directory is a minimal self-hosted `lm_node` deployment for a public Mailbox + DHT HTTP node. It is intended for early federation/bootstrap testing, not a fully audited production deployment.

## Files

- `Dockerfile` is available for local node-image development.
- `docker-compose.yml` runs the published `ghcr.io/laomou/lm-talk-node` image behind a Caddy HTTPS reverse proxy.
- `Caddyfile.example` is a TLS reverse-proxy starter config.
- `config.example.json` is a hardened starter config with:
  - persistent plaintext SQLite state under `/data` (at-rest protection via full-disk encryption / LUKS/dm-crypt);
  - bearer-token control auth;
  - mailbox quotas and rate limits;
  - JSON logs.

## One-command quick start

For a single public node with Caddy TLS, generate config/secrets and start Docker Compose with:

```bash
cd deploy/lm-node-public
./install.sh \
  --domain lm-node.example.com \
  --web-origin https://YOUR_GITHUB_USER.github.io
```

Verify the deployed node:

```bash
./verify.sh --url https://lm-node.example.com --token-file ./secrets/control-token --out lm-node-public-verify-report.json
```

Point the Web app sync service at `https://YOUR_DOMAIN` and use the value in `secrets/control-token` as the node token.

## Published container images

A release tag such as `v0.1.0` publishes matching multi-architecture (`linux/amd64`, `linux/arm64`) images:

- `ghcr.io/laomou/lm-talk-node:0.1.0` — public node service.
- `ghcr.io/laomou/lm-talk-web:0.1.0` — static Web app served by Caddy.

Stable tags additionally update `:latest`. Pin a numeric version in production:

```bash
LM_TALK_NODE_IMAGE=ghcr.io/laomou/lm-talk-node:0.1.0 docker compose up -d
docker run --rm -p 8080:80 ghcr.io/laomou/lm-talk-web:0.1.0
```

The Web image includes Caddy only for static-file delivery. Put it behind your own HTTPS Caddy site (or another reverse proxy) when exposing it publicly.

## Manual quick start

```bash
cd deploy/lm-node-public
cp config.example.json config.json
cp Caddyfile.example Caddyfile
mkdir -p secrets
openssl rand -base64 32 > secrets/control-token
chmod 600 secrets/control-token
# Edit config.json: peer_id and cors_allow_origins.
# Edit Caddyfile: replace lm-node.example.com with your node domain.
LM_TALK_NODE_IMAGE=ghcr.io/laomou/lm-talk-node:latest docker compose pull
LM_TALK_NODE_IMAGE=ghcr.io/laomou/lm-talk-node:latest docker compose up -d
```

## Production notes

- The compose template includes Caddy for TLS. Keep port `8787` internal unless you are deploying behind another HTTPS reverse proxy. Do not expose plaintext HTTP to browsers on the public internet.
- Use a full-disk-encrypted disk/volume (LUKS/dm-crypt) for `/data` to protect the plaintext `state_db` at rest.
- Keep `control_token_file` secret and rotate it using `control_previous_tokens` if needed.
- Add other public nodes to `sync_peers` for state snapshot replication.
- Set `cors_allow_origins` to the exact Web origin(s), not `*`.
- Monitor `/health`, `/control/stats`, and `/control/metrics`.


## Multi-node federation

For multiple public nodes, add peers to each node's `sync_peers` list:

```json
{
  "url": "https://peer-node.example.com",
  "peer_id": "peer-node-1",
  "token_file": "/run/secrets/peer-node-token"
}
```

Use distinct control tokens per peer where possible. Snapshot sync copies public peers, DHT records, mailbox deliveries, PreKey bundles, and consumed one-time-prekey state according to the current node sync implementation.
