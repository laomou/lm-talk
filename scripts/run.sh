#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/run.sh node [options]

Production runtime only. This script never uses `cargo run`.
It builds release and then execs target/release/lm_node.

Node options:
  --local                 bind 127.0.0.1:8787
  --lan                   bind 0.0.0.0:8787 (default)
  --ipv6                  bind [::]:8787
  --bind HOST:PORT
  --state-file PATH       default: ./lm-node-state.prd.json
  --peer-id ID            default: lm-node-prd
  --control-token TOKEN   require Authorization: Bearer TOKEN for non-health APIs
  --cors-allow-origin CSV allow only listed browser origins
  --sync-peer URL[,URL]   periodically import peer snapshots
  --sync-peer-token TOKEN Bearer token used when fetching sync peer snapshots
  --sync-interval-seconds N
USAGE
}

print_urls() {
  local bind="$1"
  local port="${bind##*:}"
  port="${port//]/}"
  echo
  echo "可在网页『我 → 消息同步 → 同步服务』填写："
  case "$bind" in
    127.0.0.1:*|localhost:*) echo "  http://127.0.0.1:$port"; return ;;
    \[::*\]|:::*) echo "  http://[::1]:$port" ;;
    0.0.0.0:*|*:*) echo "  http://127.0.0.1:$port" ;;
  esac
  if command -v hostname >/dev/null 2>&1; then
    while read -r ip; do
      [[ -z "$ip" ]] && continue
      [[ "$ip" == *:* ]] && echo "  http://[$ip]:$port" || echo "  http://$ip:$port"
    done < <(hostname -I 2>/dev/null | tr ' ' '\n' | grep -v '^127\.' | grep -v '^$' || true)
  fi
  if command -v ip >/dev/null 2>&1; then
    while read -r ifname ip6 scope; do
      [[ -z "$ip6" ]] && continue
      [[ "$scope" == "global" ]] && echo "  http://[$ip6]:$port"
      [[ "$scope" == "link" ]] && echo "  http://[$ip6%25$ifname]:$port  (IPv6 链路本地)"
    done < <(ip -o -6 addr show scope global 2>/dev/null | awk '{split($4,a,"/"); print $2, a[1], "global"}'; ip -o -6 addr show scope link 2>/dev/null | awk '{split($4,a,"/"); print $2, a[1], "link"}')
  fi
}

cmd="${1:-help}"
shift || true
case "$cmd" in
  node) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

bind="0.0.0.0:8787"
state_file="$ROOT/lm-node-state.prd.json"
peer_id="lm-node-prd"
control_token="${LM_NODE_CONTROL_TOKEN:-}"
cors_allow_origin="${LM_NODE_CORS_ALLOW_ORIGIN:-}"
sync_peer=""
sync_peer_token="${LM_NODE_SYNC_PEER_TOKEN:-}"
sync_interval_seconds="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local) bind="127.0.0.1:8787"; shift ;;
    --lan) bind="0.0.0.0:8787"; shift ;;
    --ipv6) bind="[::]:8787"; shift ;;
    --bind) bind="${2:?--bind requires HOST:PORT}"; shift 2 ;;
    --state-file) state_file="${2:?--state-file requires PATH}"; shift 2 ;;
    --peer-id) peer_id="${2:?--peer-id requires ID}"; shift 2 ;;
    --control-token) control_token="${2:?--control-token requires TOKEN}"; shift 2 ;;
    --cors-allow-origin) cors_allow_origin="${2:?--cors-allow-origin requires ORIGIN}"; shift 2 ;;
    --sync-peer) sync_peer="${2:?--sync-peer requires URL[,URL]}"; shift 2 ;;
    --sync-peer-token) sync_peer_token="${2:?--sync-peer-token requires TOKEN}"; shift 2 ;;
    --sync-interval-seconds) sync_interval_seconds="${2:?--sync-interval-seconds requires N}"; shift 2 ;;
    --debug|--release)
      echo "run.sh 是 PRD runtime，不接受 $1；固定 build --release 并执行 target/release/lm_node" >&2
      exit 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown node option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

mkdir -p "$(dirname "$state_file")"
echo "启动 LM Talk 同步服务（PRD）"
echo "绑定地址：$bind"
echo "Peer ID：$peer_id"
echo "状态文件：$state_file"
if [[ -n "$control_token" ]]; then
  echo "控制面认证：Bearer token enabled"
else
  echo "控制面认证：未配置 token，仅允许 loopback 非 health 请求"
fi
if [[ -n "$cors_allow_origin" ]]; then
  echo "CORS allow origin：$cors_allow_origin"
fi
if [[ -n "$sync_peer" && "$sync_interval_seconds" != "0" ]]; then
  echo "自动同步：$sync_peer every ${sync_interval_seconds}s"
  if [[ -n "$sync_peer_token" ]]; then
    echo "同步认证：Bearer token enabled"
  fi
fi
echo "构建：release binary"
print_urls "$bind"
echo
echo "提示：PRD 禁止 cargo run，只执行 target/release/lm_node。公网部署请放在 TLS 反向代理后。"
echo

cd "$ROOT"
cargo build --release -p lm_node
args=(serve-control --bind "$bind" --peer-id "$peer_id" --state-file "$state_file")
if [[ -n "$control_token" ]]; then
  args+=(--control-token "$control_token")
fi
if [[ -n "$cors_allow_origin" ]]; then
  args+=(--cors-allow-origin "$cors_allow_origin")
fi
if [[ -n "$sync_peer" ]]; then
  args+=(--sync-peer "$sync_peer")
fi
if [[ -n "$sync_peer_token" ]]; then
  args+=(--sync-peer-token "$sync_peer_token")
fi
if [[ "$sync_interval_seconds" != "0" ]]; then
  args+=(--sync-interval-seconds "$sync_interval_seconds")
fi
exec "$ROOT/target/release/lm_node" "${args[@]}"
