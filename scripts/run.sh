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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local) bind="127.0.0.1:8787"; shift ;;
    --lan) bind="0.0.0.0:8787"; shift ;;
    --ipv6) bind="[::]:8787"; shift ;;
    --bind) bind="${2:?--bind requires HOST:PORT}"; shift 2 ;;
    --state-file) state_file="${2:?--state-file requires PATH}"; shift 2 ;;
    --peer-id) peer_id="${2:?--peer-id requires ID}"; shift 2 ;;
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
echo "构建：release binary"
print_urls "$bind"
echo
echo "提示：PRD 禁止 cargo run，只执行 target/release/lm_node。公网部署请放在 TLS 反向代理后。"
echo

cd "$ROOT"
cargo build --release -p lm_node
exec "$ROOT/target/release/lm_node" serve-control --bind "$bind" --peer-id "$peer_id" --state-file "$state_file"
