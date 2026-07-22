#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
One-command LM Talk public node installer.

Usage:
  deploy/lm-node-public/install.sh --domain node.example.com --web-origin https://your-web.example.com [options]

Options:
  --domain DOMAIN        Public HTTPS domain for this node. Required.
  --web-origin ORIGIN    Browser app origin allowed by CORS. May be repeated. Required unless --allow-any-origin.
  --peer-id ID           Node peer id. Default: derived from domain.
  --data-dir DIR         Deployment directory. Default: current deploy/lm-node-public directory.
  --email EMAIL          ACME contact email for Caddy. Optional.
  --image IMAGE          Node image. Default: ghcr.io/laomou/lm-talk-node:latest.
  --allow-any-origin    Use CORS ["*"] for testing only; not recommended for production.
  --dry-run             Generate files and print next steps without running docker compose.
  -h, --help            Show this help.

Examples:
  ./install.sh --domain node.example.com --web-origin https://laomou.github.io
  ./install.sh --domain node.example.com --web-origin https://app.example.com --data-dir /opt/lm-talk-node
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOMAIN=""
PEER_ID=""
DATA_DIR="$SCRIPT_DIR"
EMAIL=""
IMAGE="ghcr.io/laomou/lm-talk-node:latest"
DRY_RUN=0
ALLOW_ANY_ORIGIN=0
WEB_ORIGINS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --domain) DOMAIN="${2:-}"; shift 2 ;;
    --web-origin) WEB_ORIGINS+=("${2:-}"); shift 2 ;;
    --peer-id) PEER_ID="${2:-}"; shift 2 ;;
    --data-dir) DATA_DIR="${2:-}"; shift 2 ;;
    --email) EMAIL="${2:-}"; shift 2 ;;
    --image) IMAGE="${2:-}"; shift 2 ;;
    --allow-any-origin) ALLOW_ANY_ORIGIN=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$DOMAIN" ]]; then
  echo "error: --domain is required" >&2
  usage >&2
  exit 2
fi
if [[ "$ALLOW_ANY_ORIGIN" != "1" && "${#WEB_ORIGINS[@]}" -eq 0 ]]; then
  echo "error: at least one --web-origin is required, or use --allow-any-origin for testing" >&2
  usage >&2
  exit 2
fi
if [[ -z "$PEER_ID" ]]; then
  PEER_ID="lm-node-${DOMAIN//[^A-Za-z0-9]/-}"
fi

if [[ "$DRY_RUN" != "1" ]]; then
  if ! command -v docker >/dev/null 2>&1; then
    echo "error: docker is required" >&2
    exit 127
  fi
  if docker compose version >/dev/null 2>&1; then
    COMPOSE=(docker compose)
  elif command -v docker-compose >/dev/null 2>&1; then
    COMPOSE=(docker-compose)
  else
    echo "error: docker compose plugin or docker-compose is required" >&2
    exit 127
  fi
fi

mkdir -p "$DATA_DIR" "$DATA_DIR/secrets"

copy_template() {
  local src="$1" dst="$2"
  if [[ "$src" != "$dst" ]]; then
    cp "$src" "$dst"
  fi
}
copy_template "$SCRIPT_DIR/docker-compose.yml" "$DATA_DIR/docker-compose.yml"

secret_file() {
  local path="$1"
  if [[ ! -s "$path" ]]; then
    openssl rand -base64 32 > "$path"
    chmod 600 "$path"
  fi
}
secret_file "$DATA_DIR/secrets/control-token"

python3 - <<'PY' "$SCRIPT_DIR/config.example.json" "$DATA_DIR/config.json" "$DOMAIN" "$PEER_ID" "$ALLOW_ANY_ORIGIN" "${WEB_ORIGINS[@]}"
import json, pathlib, sys
src, dst, domain, peer_id, allow_any, *origins = sys.argv[1:]
config = json.loads(pathlib.Path(src).read_text())
config['peer_id'] = peer_id
config['cors_allow_origins'] = ['*'] if allow_any == '1' else origins
pathlib.Path(dst).write_text(json.dumps(config, indent=2) + '\n')
PY

{
  if [[ -n "$EMAIL" ]]; then
    echo "{"
    echo "  email $EMAIL"
    echo "}"
    echo
  fi
  cat <<EOF_CADDY
$DOMAIN {
  encode zstd gzip

  header {
    Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
    X-Content-Type-Options "nosniff"
    Referrer-Policy "no-referrer"
  }

  reverse_proxy lm-node:8787 {
    header_up Host {host}
    header_up X-Forwarded-Proto {scheme}
    header_up X-Forwarded-For {remote_host}
  }
}
EOF_CADDY
} > "$DATA_DIR/Caddyfile"

cat > "$DATA_DIR/.env" <<EOF_ENV
LM_TALK_NODE_IMAGE=$IMAGE
EOF_ENV

CONTROL_TOKEN="$(cat "$DATA_DIR/secrets/control-token")"
cat > "$DATA_DIR/DEPLOYMENT_INFO.txt" <<EOF_INFO
LM Talk node deployment
Domain: https://$DOMAIN
Peer ID: $PEER_ID
Config: $DATA_DIR/config.json
Caddyfile: $DATA_DIR/Caddyfile
Control token file: $DATA_DIR/secrets/control-token
Web sync service URL: https://$DOMAIN
Web sync token: $CONTROL_TOKEN
EOF_INFO
chmod 600 "$DATA_DIR/DEPLOYMENT_INFO.txt"

if [[ "$DRY_RUN" == "1" ]]; then
  echo "== dry run complete =="
  echo "generated deployment in: $DATA_DIR"
else
  echo "== starting LM Talk node =="
  (cd "$DATA_DIR" && "${COMPOSE[@]}" pull && "${COMPOSE[@]}" up -d)
fi

cat <<EOF_DONE

LM Talk node deployment prepared.

Node URL: https://$DOMAIN
Peer ID: $PEER_ID
Control token file: $DATA_DIR/secrets/control-token
Web app settings:
  Sync service URL: https://$DOMAIN
  Token: $CONTROL_TOKEN

Verify:
  $DATA_DIR/verify.sh --url https://$DOMAIN --token-file $DATA_DIR/secrets/control-token
EOF_DONE

if [[ ! -f "$DATA_DIR/verify.sh" ]]; then
  cp "$SCRIPT_DIR/verify.sh" "$DATA_DIR/verify.sh" 2>/dev/null || true
  chmod +x "$DATA_DIR/verify.sh" 2>/dev/null || true
fi
