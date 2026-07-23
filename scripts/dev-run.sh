#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/dev-run.sh node [options]
  ./scripts/dev-run.sh web [options]

Build the current lm_node release binary, package it into a local Docker image,
and restart the Node container. HTTPS is provided by the lm-talk-web Caddy
container through https://<LAN-IP>/node/; this script does not publish port 8787.

Options:
  --config-file PATH       Node JSON configuration (required unless LM_NODE_CONFIG_FILE is set)
  --data-dir PATH          Persistent /data mount (default: sibling data directory)
  --container-name NAME    Docker container name (default: lm-talk-node)
  --network NAME           Docker network shared with Caddy (default: lm-talk-lan)
  --image-tag TAG          Local image tag (default: lm-talk-node:dev)
  --public-url URL         HTTPS origin served by Caddy, used only when printing the sync address
  --no-build               Reuse the current local image; only recreate the container
  --logs                   Follow Node logs after startup
  --check-config           Validate local prerequisites and exit
  -h, --help               Show this help

Example:
  ./scripts/dev-run.sh node \
    --config-file /home/user/lm-talk-node/config.json \
    --data-dir /home/user/lm-talk-node/data \
    --public-url https://lm-talk.lan

The Caddy container must be on the same Docker network and proxy /node/* to
lm-talk-node:8787. --public-url is deployment-specific: it must exactly match
the HTTPS host configured in Caddy and included in cors_allow_origins.

Web options:
  --public-url URL         HTTPS origin to serve (required unless --caddyfile is used)
  --caddyfile PATH         Existing Caddyfile to mount instead of generating one
  --caddy-data-dir PATH    Persistent Caddy data and generated config directory
  --root-cert PATH         Export the active Caddy root CA to this host path after startup
  --node-container NAME    Node container Caddy proxies to (default: lm-talk-node)
  --container-name NAME    Docker container name (default: lm-talk-web)
  --network NAME           Docker network shared with Node (default: lm-talk-lan)
  --image-tag TAG          Local image tag (default: lm-talk-web:dev)
  --no-build               Reuse the current local image; only recreate the container
  --logs                   Follow Caddy logs after startup
  --check-config           Validate local prerequisites and exit

Example:
  ./scripts/dev-run.sh web \
    --public-url https://lm-talk.lan \
    --caddy-data-dir /home/user/lm-talk-web/caddy-data \
    --root-cert /home/user/lm-talk-web/lm-talk-local-root.crt
USAGE
}

cmd="${1:-help}"
shift || true
case "$cmd" in
  node|web) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "未找到 docker，请先安装并启动 Docker。" >&2
    exit 2
  fi
  if ! docker info >/dev/null 2>&1; then
    echo "Docker daemon 不可用，请确认 Docker 正在运行。" >&2
    exit 2
  fi
}

ensure_network() {
  local name="$1"
  if ! docker network inspect "$name" >/dev/null 2>&1; then
    echo "创建 Docker 网络：$name"
    docker network create "$name" >/dev/null
  fi
}

normalize_public_url() {
  local value="$1"
  value="${value%/}"
  if [[ ! "$value" =~ ^https://[^/]+$ ]]; then
    echo "--public-url 必须是 HTTPS origin，例如 https://lm-talk.lan 或 https://10.0.0.8。" >&2
    exit 2
  fi
  printf '%s' "$value"
}

run_web() {
  local public_url="${LM_TALK_PUBLIC_URL:-}"
  local caddyfile=""
  local caddy_data_dir="${LM_TALK_CADDY_DATA_DIR:-}"
  local root_cert="${LM_TALK_ROOT_CERT:-}"
  local node_container="${LM_NODE_CONTAINER_NAME:-lm-talk-node}"
  local container_name="${LM_WEB_CONTAINER_NAME:-lm-talk-web}"
  local network_name="${LM_NODE_DOCKER_NETWORK:-lm-talk-lan}"
  local image_tag="${LM_WEB_IMAGE_TAG:-lm-talk-web:dev}"
  local build_image=1
  local follow_logs=0
  local check_config=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --public-url) public_url="${2:?--public-url requires URL}"; shift 2 ;;
      --caddyfile) caddyfile="${2:?--caddyfile requires PATH}"; shift 2 ;;
      --caddy-data-dir) caddy_data_dir="${2:?--caddy-data-dir requires PATH}"; shift 2 ;;
      --root-cert) root_cert="${2:?--root-cert requires PATH}"; shift 2 ;;
      --node-container) node_container="${2:?--node-container requires NAME}"; shift 2 ;;
      --container-name) container_name="${2:?--container-name requires NAME}"; shift 2 ;;
      --network) network_name="${2:?--network requires NAME}"; shift 2 ;;
      --image-tag) image_tag="${2:?--image-tag requires TAG}"; shift 2 ;;
      --no-build) build_image=0; shift ;;
      --logs) follow_logs=1; shift ;;
      --check-config) check_config=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) echo "unknown web option: $1" >&2; usage >&2; exit 2 ;;
    esac
  done

  if [[ -z "$caddyfile" && -z "$public_url" ]]; then
    echo "web 启动需要 --public-url URL，或提供已有的 --caddyfile PATH。" >&2
    exit 2
  fi
  if [[ -n "$public_url" ]]; then
    public_url="$(normalize_public_url "$public_url")"
  fi
  if [[ -n "$caddyfile" && ! -f "$caddyfile" ]]; then
    echo "Caddyfile 不存在：$caddyfile" >&2
    exit 2
  fi
  if [[ -z "$caddy_data_dir" ]]; then
    caddy_data_dir="$ROOT/.local/lm-talk-web/caddy-data"
  fi
  mkdir -p "$caddy_data_dir"
  caddy_data_dir="$(cd "$caddy_data_dir" && pwd)"
  if [[ -n "$root_cert" ]]; then
    mkdir -p "$(dirname "$root_cert")"
    root_cert="$(cd "$(dirname "$root_cert")" && pwd)/$(basename "$root_cert")"
  fi
  if [[ -z "$caddyfile" ]]; then
    caddyfile="$caddy_data_dir/Caddyfile"
    cat > "$caddyfile" <<EOF
{
  servers {
    protocols h1 h2
  }
}

$public_url {
  tls internal

  handle_path /node/* {
    reverse_proxy $node_container:8787
  }

  handle_path /admin/* {
    root * /admin
    encode zstd gzip
    try_files {path} /index.html
    file_server
  }

  handle {
    root * /srv
    encode zstd gzip
    try_files {path} /index.html
    file_server
  }
}
EOF
  fi
  caddyfile="$(cd "$(dirname "$caddyfile")" && pwd)/$(basename "$caddyfile")"

  require_docker
  ensure_network "$network_name"
  echo "Caddyfile：$caddyfile"
  echo "Caddy 数据：$caddy_data_dir"
  [[ -n "$root_cert" ]] && echo "根证书导出：$root_cert"
  echo "Docker 网络：$network_name"
  echo "容器名称：$container_name"
  [[ -n "$public_url" ]] && echo "HTTPS 来源：$public_url"
  if [[ "$check_config" == "1" ]]; then
    echo "配置检查：OK"
    exit 0
  fi

  if [[ "$build_image" == "1" ]]; then
    local build_ref
    build_ref="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo container)"
    echo "构建 Web Docker 镜像"
    docker build \
      -f "$ROOT/docker/web/Dockerfile" \
      --build-arg "BUILD_REF=$build_ref" \
      -t "$image_tag" \
      "$ROOT"
  fi
  if ! docker image inspect "$image_tag" >/dev/null 2>&1; then
    echo "本地镜像不存在：$image_tag；请移除 --no-build 后重试。" >&2
    exit 2
  fi

  docker rm -f "$container_name" >/dev/null 2>&1 || true
  docker run -d \
    --name "$container_name" \
    --restart unless-stopped \
    --network "$network_name" \
    -p 80:80 \
    -p 443:443 \
    -p 443:443/udp \
    -v "$caddyfile:/etc/caddy/Caddyfile:ro" \
    -v "$caddy_data_dir:/data" \
    "$image_tag" >/dev/null

  sleep 1
  if [[ "$(docker inspect -f '{{.State.Running}}' "$container_name")" != "true" ]]; then
    echo "Web/Caddy 容器启动失败，最近日志：" >&2
    docker logs --tail 80 "$container_name" >&2 || true
    exit 1
  fi

  echo "Web Docker 容器已启动：$container_name ($image_tag)"
  if [[ -n "$root_cert" ]]; then
    # This only copies the root CA Caddy is actively using from the mounted
    # data directory. It does not create or rotate a CA by itself.
    if ! docker cp "$container_name:/data/caddy/pki/authorities/local/root.crt" "$root_cert"; then
      echo "无法导出当前 Caddy 根证书，最近日志：" >&2
      docker logs --tail 80 "$container_name" >&2 || true
      exit 1
    fi
    chmod 0644 "$root_cert"
    echo "当前 Caddy 根证书已导出：$root_cert"
  fi
  if [[ -n "$public_url" ]]; then
    echo "打开：$public_url/"
    echo "复用已挂载的 Caddy 数据目录中的现有 HTTPS 证书和 CA。"
  fi
  if [[ "$follow_logs" == "1" ]]; then
    exec docker logs -f "$container_name"
  fi
}

if [[ "$cmd" == "web" ]]; then
  run_web "$@"
  exit 0
fi

config_file="${LM_NODE_CONFIG_FILE:-}"
data_dir="${LM_NODE_DATA_DIR:-}"
container_name="${LM_NODE_CONTAINER_NAME:-lm-talk-node}"
network_name="${LM_NODE_DOCKER_NETWORK:-lm-talk-lan}"
image_tag="${LM_NODE_IMAGE_TAG:-lm-talk-node:dev}"
public_url="${LM_TALK_PUBLIC_URL:-}"
build_image=1
follow_logs=0
check_config=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-file) config_file="${2:?--config-file requires PATH}"; shift 2 ;;
    --data-dir) data_dir="${2:?--data-dir requires PATH}"; shift 2 ;;
    --container-name) container_name="${2:?--container-name requires NAME}"; shift 2 ;;
    --network) network_name="${2:?--network requires NAME}"; shift 2 ;;
    --image-tag) image_tag="${2:?--image-tag requires TAG}"; shift 2 ;;
    --public-url) public_url="${2:?--public-url requires URL}"; shift 2 ;;
    --no-build) build_image=0; shift ;;
    --logs) follow_logs=1; shift ;;
    --check-config) check_config=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown node option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$config_file" ]]; then
  echo "需要 --config-file PATH（或设置 LM_NODE_CONFIG_FILE）。" >&2
  exit 2
fi

if [[ -n "$public_url" ]]; then
  public_url="${public_url%/}"
  if [[ ! "$public_url" =~ ^https://[^/]+$ ]]; then
    echo "--public-url 必须是 HTTPS origin，例如 https://lm-talk.lan 或 https://10.0.0.8。" >&2
    exit 2
  fi
fi

if [[ ! -f "$config_file" ]]; then
  echo "Node 配置文件不存在：$config_file" >&2
  exit 2
fi

config_file="$(cd "$(dirname "$config_file")" && pwd)/$(basename "$config_file")"
read_control_token() {
  python3 - "$config_file" <<'PY'
import json
import os
import sys

config_path = sys.argv[1]
with open(config_path, encoding="utf-8") as source:
    config = json.load(source)

token = str(config.get("control_token") or "").strip()
if not token:
    token_file = str(config.get("control_token_file") or "").strip()
    if token_file:
        if not os.path.isabs(token_file):
            token_file = os.path.join(os.path.dirname(config_path), token_file)
        with open(token_file, encoding="utf-8") as source:
            token = source.read().strip()

print(token)
PY
}

control_token="$(read_control_token)"
if [[ -z "$control_token" ]]; then
  cat >&2 <<'WARN'
警告：配置中未读取到 control_token 或 control_token_file。
Node 会拒绝非 loopback 的 API 请求；请在配置中设置强随机 control token 后再用于局域网。
WARN
fi

if [[ -z "$data_dir" ]]; then
  data_dir="$(dirname "$config_file")/data"
fi
mkdir -p "$data_dir"
data_dir="$(cd "$data_dir" && pwd)"

if ! command -v docker >/dev/null 2>&1; then
  echo "未找到 docker，请先安装并启动 Docker。" >&2
  exit 2
fi
if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon 不可用，请确认 Docker 正在运行。" >&2
  exit 2
fi

if ! docker network inspect "$network_name" >/dev/null 2>&1; then
  echo "创建 Docker 网络：$network_name"
  docker network create "$network_name" >/dev/null
fi

if ! docker ps --format '{{.Names}}' | grep -Fxq 'lm-talk-web'; then
  cat >&2 <<'WARN'
警告：未发现运行中的 lm-talk-web Caddy 容器。
Node 仍会启动，但浏览器无法通过 HTTPS /node/ 访问它。请先启动 Web/Caddy，
并确保其加入同一个 Docker network，且配置了：
  handle_path /node/* { reverse_proxy lm-talk-node:8787 }
WARN
fi

echo "Node 配置：$config_file"
echo "Node 数据：$data_dir"
echo "Docker 网络：$network_name"
echo "容器名称：$container_name"
echo "HTTPS：由 lm-talk-web Caddy 通过 /node/ 反向代理提供；不暴露宿主机 8787。"
if [[ -n "$public_url" ]]; then
  echo "HTTPS 来源：$public_url"
else
  echo "HTTPS 来源：未指定；启动后不会猜测局域网 IP。可传 --public-url https://<当前部署主机>。"
fi

if [[ "$check_config" == "1" ]]; then
  echo "配置检查：OK"
  exit 0
fi

if [[ "$build_image" == "1" ]]; then
  echo "构建 lm_node release binary"
  (
    cd "$ROOT"
    cargo build --release -p lm_node
    mkdir -p docker/node/dist
    install -m 0755 target/release/lm_node docker/node/dist/lm_node-linux-x86_64
    docker build -f docker/node/Dockerfile -t "$image_tag" docker/node
  )
fi

if ! docker image inspect "$image_tag" >/dev/null 2>&1; then
  echo "本地镜像不存在：$image_tag；请移除 --no-build 后重试。" >&2
  exit 2
fi

docker rm -f "$container_name" >/dev/null 2>&1 || true
docker run -d \
  --name "$container_name" \
  --restart unless-stopped \
  --network "$network_name" \
  --user "$(id -u):$(id -g)" \
  -v "$config_file:/app/config.json:ro" \
  -v "$data_dir:/data" \
  "$image_tag" >/dev/null

sleep 1
if [[ "$(docker inspect -f '{{.State.Running}}' "$container_name")" != "true" ]]; then
  echo "Node 容器启动失败，最近日志：" >&2
  docker logs --tail 80 "$container_name" >&2 || true
  exit 1
fi

echo "Node Docker 容器已启动：$container_name ($image_tag)"
if [[ -n "$public_url" ]]; then
  if [[ -n "$control_token" ]]; then
    echo "请在 Web 中填写（含访问令牌，请仅粘贴到可信设备）：$public_url/node|$control_token"
  else
    echo "请在 Web 中填写：$public_url/node|<control-token>"
  fi
else
  echo "请在 Web 中填写：<你的 Caddy HTTPS 地址>/node|<control-token>"
fi
if [[ "$follow_logs" == "1" ]]; then
  exec docker logs -f "$container_name"
fi
