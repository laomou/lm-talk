#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIND="0.0.0.0:8787"
STATE_FILE="$ROOT/lm-node-state.json"
RELEASE=0

usage() {
  cat <<'USAGE'
启动 LM Talk 同步服务（lm_node）。

用法：
  ./scripts/start-node.sh [选项]

常用：
  ./scripts/start-node.sh                 # 局域网 IPv4: 0.0.0.0:8787
  ./scripts/start-node.sh --local         # 仅本机: 127.0.0.1:8787
  ./scripts/start-node.sh --ipv6          # IPv6: [::]:8787，并显示 IPv6 URL
  ./scripts/start-node.sh --bind '[::]:8787'
  ./scripts/start-node.sh --state-file ./lm-node-state.json
  ./scripts/start-node.sh --release

网页里填写：
  本机:     http://127.0.0.1:8787
  局域网:   http://你的局域网IP:8787
  IPv6:     http://[你的IPv6地址]:8787
  链路本地: http://[fe80::xxxx%25网卡名]:8787
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --local)
      BIND="127.0.0.1:8787"
      shift
      ;;
    --lan)
      BIND="0.0.0.0:8787"
      shift
      ;;
    --ipv6)
      BIND="[::]:8787"
      shift
      ;;
    --bind)
      BIND="${2:?--bind 需要 host:port}"
      shift 2
      ;;
    --state-file)
      STATE_FILE="$2"
      shift 2
      ;;
    --release)
      RELEASE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "未知参数：$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

print_detected_urls() {
  local port="${BIND##*:}"
  port="${port//]/}"

  echo
  echo "可在网页『我 → 消息同步 → 同步服务』填写："

  case "$BIND" in
    127.0.0.1:*|localhost:*)
      echo "  http://127.0.0.1:$port"
      return
      ;;
    \[::*\]|:::*)
      echo "  http://[::1]:$port"
      ;;
    0.0.0.0:*|*:*)
      echo "  http://127.0.0.1:$port"
      ;;
  esac

  if command -v hostname >/dev/null 2>&1; then
    while read -r ip; do
      [[ -z "$ip" ]] && continue
      if [[ "$ip" == *:* ]]; then
        # 跳过临时/链路本地展示过多，仅给出可复制格式。
        echo "  http://[$ip]:$port"
      else
        echo "  http://$ip:$port"
      fi
    done < <(hostname -I 2>/dev/null | tr ' ' '\n' | grep -v '^127\.' | grep -v '^$' || true)
  fi

  if command -v ip >/dev/null 2>&1; then
    local printed_v6=0
    while read -r ifname ip6 scope; do
      [[ -z "$ip6" ]] && continue
      if [[ "$scope" == "global" ]]; then
        echo "  http://[$ip6]:$port"
        printed_v6=1
      elif [[ "$scope" == "link" ]]; then
        # 链路本地 IPv6 需要带网卡 zone id；URL 里 % 要写成 %25。
        echo "  http://[$ip6%25$ifname]:$port  (IPv6 链路本地)"
        printed_v6=1
      fi
    done < <(ip -o -6 addr show scope global 2>/dev/null | awk '{split($4,a,"/"); print $2, a[1], "global"}'; ip -o -6 addr show scope link 2>/dev/null | awk '{split($4,a,"/"); print $2, a[1], "link"}')
    if [[ "$printed_v6" == "0" ]]; then
      echo "  未检测到可用 IPv6 地址"
    fi
    if [[ "$BIND" != \[* && "$BIND" != ::* ]]; then
      echo "  注意：当前绑定 $BIND 主要用于 IPv4；如需 IPv6 请用 ./scripts/start-node.sh --ipv6"
    fi
  fi
}

echo "启动 LM Talk 同步服务"
echo "绑定地址：$BIND"
echo "状态文件：$STATE_FILE"
print_detected_urls

echo
echo "提示："
echo "  - 局域网设备需要防火墙放行端口 8787。"
echo "  - IPv6 URL 必须带方括号，例如 http://[2408:...:c20]:8787。"
echo "  - 如果从 GitHub Pages(HTTPS) 访问本机/局域网 HTTP 服务，浏览器可能要求使用最新版 lm_node。"
echo

cd "$ROOT"
if [[ "$RELEASE" == "1" ]]; then
  exec cargo run --release -p lm_node -- serve-control --bind "$BIND" --state-file "$STATE_FILE"
else
  exec cargo run -p lm_node -- serve-control --bind "$BIND" --state-file "$STATE_FILE"
fi
