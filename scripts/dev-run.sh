#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/dev-run.sh node [options]

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
USAGE
}

cmd="${1:-help}"
shift || true
case "$cmd" in
  node) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

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
  echo "请在 Web 中填写：$public_url/node|<control-token>"
else
  echo "请在 Web 中填写：<你的 Caddy HTTPS 地址>/node|<control-token>"
fi
if [[ "$follow_logs" == "1" ]]; then
  exec docker logs -f "$container_name"
fi
