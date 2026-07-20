#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"
NETWORK="${LM_NODE_FEDERATION_DOCKER_NETWORK:-lm-talk-fed}"
IMAGE="${LM_NODE_FEDERATION_IMAGE:-lm-talk/lm-node-public:local}"

has_compose() { docker compose version >/dev/null 2>&1; }
use_native_compose() { [[ "${LM_NODE_FEDERATION_DIRECT_DOCKER:-0}" != "1" ]] && has_compose; }

ensure_secrets() {
  mkdir -p "$ROOT/secrets"
  local n
  for n in a b c; do
    [[ -s "$ROOT/secrets/node-$n-token" ]] || openssl rand -base64 32 > "$ROOT/secrets/node-$n-token"
    [[ -s "$ROOT/secrets/state-db-passphrase-$n" ]] || openssl rand -base64 32 > "$ROOT/secrets/state-db-passphrase-$n"
    [[ -s "$ROOT/secrets/state-file-passphrase-$n" ]] || openssl rand -base64 32 > "$ROOT/secrets/state-file-passphrase-$n"
  done
  chmod 600 "$ROOT"/secrets/* 2>/dev/null || true
}

write_direct_configs() {
  mkdir -p "$ROOT/.docker-run"
  python3 - <<'PY' "$ROOT"
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
for n in "abc":
    cfg = json.loads((root / f"node-{n}.json").read_text())
    for peer in cfg.get("sync_peers", []):
        url = peer.get("url", "")
        if url.startswith("http://caddy-"):
            peer["url"] = "http://node-" + url.split("http://caddy-", 1)[1] + ":8787"
    # Direct-docker fallback runs all nodes on published host ports and uses the
    # smoke scripts' explicit snapshot imports. Disable background sync/DHT
    # runners here to avoid noisy self-load while compose networking is absent.
    cfg["sync_interval_seconds"] = 0
    cfg["dht_replication_factor"] = 0
    cfg["dht_routing_refresh_max_targets"] = 0
    (root / ".docker-run" / f"node-{n}.json").write_text(json.dumps(cfg, indent=2) + "\n")
PY
}

ensure_image() {
  local force_build="${1:-0}"
  if [[ "$force_build" != "1" ]] && docker image inspect "$IMAGE" >/dev/null 2>&1; then return 0; fi
  docker build -f "$REPO_ROOT/deploy/lm-node-public/Dockerfile" -t "$IMAGE" "$REPO_ROOT"
}

wait_direct_health() {
  local port
  for _ in $(seq 1 80); do
    local ok=1
    for port in 8081 8082 8083; do
      curl -fsS "http://127.0.0.1:$port/health" >/dev/null 2>&1 || ok=0
    done
    [[ "$ok" == "1" ]] && return 0
    sleep 0.25
  done
  docker ps -a --filter name=node- --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}' >&2 || true
  for n in a b c; do docker logs --tail=80 "node-$n" >&2 || true; done
  return 1
}

direct_up() {
  local force_build=0
  for arg in "$@"; do [[ "$arg" == "--build" ]] && force_build=1; done
  ensure_secrets
  write_direct_configs
  ensure_image "$force_build"
  docker network inspect "$NETWORK" >/dev/null 2>&1 || docker network create "$NETWORK" >/dev/null
  mkdir -p "$ROOT/.docker-data/node-a" "$ROOT/.docker-data/node-b" "$ROOT/.docker-data/node-c"
  local n port
  for n in a b c; do
    case "$n" in a) port=8081 ;; b) port=8082 ;; c) port=8083 ;; esac
    # Recreate containers on every direct `up` so generated configs and newly
    # built binaries are always applied; persistent data remains in .docker-data.
    docker rm -f "node-$n" >/dev/null 2>&1 || true
    docker run -d --name "node-$n" --network "$NETWORK" -p "$port:8787" \
      -v "$ROOT/.docker-run/node-$n.json:/app/config.json:ro" \
      -v "$ROOT/secrets:/run/secrets:ro" \
      -v "$ROOT/.docker-data/node-$n:/data" \
      "$IMAGE" serve-control --config-file /app/config.json >/dev/null
  done
  wait_direct_health
}

direct_exec() {
  [[ "${1:-}" == "-T" ]] && shift
  local svc="$1"; shift
  local args=("$@")
  local i
  for i in "${!args[@]}"; do
    if [[ "${args[$i]}" == /tmp/* && -f "${args[$i]}" ]]; then
      local dest="/tmp/compose-wrapper-$(basename "${args[$i]}")"
      docker cp "${args[$i]}" "$svc:$dest" >/dev/null
      args[$i]="$dest"
    fi
  done
  exec docker exec -i "$svc" "${args[@]}"
}

direct_stop() {
  local svc
  for svc in "$@"; do
    [[ "$svc" == caddy-* ]] && continue
    docker stop "$svc" >/dev/null 2>&1 || true
  done
}

direct_down() {
  docker rm -f node-a node-b node-c caddy-a caddy-b caddy-c >/dev/null 2>&1 || true
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
}

if use_native_compose; then
  exec docker compose -f "$ROOT/docker-compose.yml" "$@"
fi

case "${1:-}" in
  up)
    shift
    direct_up "$@"
    ;;
  exec)
    shift
    direct_exec "$@"
    ;;
  stop)
    shift
    direct_stop "$@"
    ;;
  down)
    shift
    direct_down
    ;;
  ps)
    docker ps --filter name=node- --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
    ;;
  logs)
    shift || true
    if [[ "$#" -eq 0 ]]; then set -- node-a node-b node-c; fi
    for svc in "$@"; do docker logs "$svc"; done
    ;;
  *)
    echo "docker compose plugin not found and direct fallback does not support: $*" >&2
    echo "Install docker compose plugin or set LM_NODE_FEDERATION_DIRECT_DOCKER=1 for supported commands." >&2
    exit 2
    ;;
esac
