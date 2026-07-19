#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/dev-run.sh node [options]

Production runtime only. This script never uses `cargo run`.
It builds release and then execs target/release/lm_node.

Node options:
  --local                 bind 127.0.0.1:8787
  --lan                   bind 0.0.0.0:8787 (default)
  --ipv6                  bind [::]:8787
  --config-file PATH      JSON config for lm_node serve-control
  --bind HOST:PORT
  --state-db PATH         default: ./lm-node-state.prd.sqlite3 unless --config-file is used
  --state-db-require-encryption true|false fail closed if DB encryption is required (sqlcipher built in by default)
  --state-file PATH       optional legacy JSON snapshot state file
  --peer-id ID            default: lm-node-prd
  --control-token TOKEN   require Authorization: Bearer TOKEN for non-health APIs
  --control-token-file PATH read control token from file
  --cors-allow-origin CSV allow only listed browser origins
  --sync-peer URL[,URL]   periodically import peer snapshots
  --sync-peer-token TOKEN Bearer token used when fetching sync peer snapshots
  --sync-peer-token-file PATH read sync peer token from file
  --sync-interval-seconds N
  --sync-max-backoff-seconds N maximum retry backoff after sync failures (default: 300)
  --dht-replication-factor N DHT StoreRecord replication factor (default: 3)
  --dht-routing-refresh-limit N DHT FindNode response limit (default: 8)
  --dht-routing-refresh-max-targets N DHT refresh targets per sync run (default: 8)
  --dht-transport http-control|libp2p DHT runner transport (default: http-control)
  --dht-peer-quarantine-consecutive-failures N skip DHT peers in backoff after N consecutive failures (default: 5; 0 disables)
  --rate-limit-window-seconds N per-client control API rate window (default: 60; 0 disables)
  --rate-limit-max-requests N max non-health requests per client/window (default: 600; 0 disables)
  --log-format text|json  stdout log format (default: text; env: LM_NODE_LOG_FORMAT)
  --check-config          validate script options and exit before building/running
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

config_file="${LM_NODE_CONFIG_FILE:-}"
bind="0.0.0.0:8787"
bind_set=0
state_file="$ROOT/lm-node-state.prd.json"
state_file_set=0
state_db="$ROOT/lm-node-state.prd.sqlite3"
state_db_set=0
state_db_require_encryption="${LM_NODE_STATE_DB_REQUIRE_ENCRYPTION:-false}"
state_db_require_encryption_set=$([[ -n "${LM_NODE_STATE_DB_REQUIRE_ENCRYPTION:-}" ]] && echo 1 || echo 0)
peer_id="lm-node-prd"
peer_id_set=0
control_token="${LM_NODE_CONTROL_TOKEN:-}"
control_token_file="${LM_NODE_CONTROL_TOKEN_FILE:-}"
cors_allow_origin="${LM_NODE_CORS_ALLOW_ORIGIN:-}"
sync_peer=""
sync_peer_token="${LM_NODE_SYNC_PEER_TOKEN:-}"
sync_peer_token_file="${LM_NODE_SYNC_PEER_TOKEN_FILE:-}"
sync_interval_seconds="0"
sync_max_backoff_seconds="300"
dht_replication_factor="${LM_NODE_DHT_REPLICATION_FACTOR:-3}"
dht_replication_factor_set=$([[ -n "${LM_NODE_DHT_REPLICATION_FACTOR:-}" ]] && echo 1 || echo 0)
dht_routing_refresh_limit="${LM_NODE_DHT_ROUTING_REFRESH_LIMIT:-8}"
dht_routing_refresh_limit_set=$([[ -n "${LM_NODE_DHT_ROUTING_REFRESH_LIMIT:-}" ]] && echo 1 || echo 0)
dht_routing_refresh_max_targets="${LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS:-8}"
dht_routing_refresh_max_targets_set=$([[ -n "${LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS:-}" ]] && echo 1 || echo 0)
dht_transport="${LM_NODE_DHT_TRANSPORT:-http-control}"
dht_transport_set=$([[ -n "${LM_NODE_DHT_TRANSPORT:-}" ]] && echo 1 || echo 0)
dht_peer_quarantine_consecutive_failures="${LM_NODE_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES:-5}"
dht_peer_quarantine_consecutive_failures_set=$([[ -n "${LM_NODE_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES:-}" ]] && echo 1 || echo 0)
rate_limit_window_seconds="${LM_NODE_RATE_LIMIT_WINDOW_SECONDS:-60}"
rate_limit_window_seconds_set=$([[ -n "${LM_NODE_RATE_LIMIT_WINDOW_SECONDS:-}" ]] && echo 1 || echo 0)
rate_limit_max_requests="${LM_NODE_RATE_LIMIT_MAX_REQUESTS:-600}"
rate_limit_max_requests_set=$([[ -n "${LM_NODE_RATE_LIMIT_MAX_REQUESTS:-}" ]] && echo 1 || echo 0)
log_format="${LM_NODE_LOG_FORMAT:-text}"
log_format_set=$([[ -n "${LM_NODE_LOG_FORMAT:-}" ]] && echo 1 || echo 0)
check_config=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-file) config_file="${2:?--config-file requires PATH}"; shift 2 ;;
    --local) bind="127.0.0.1:8787"; bind_set=1; shift ;;
    --lan) bind="0.0.0.0:8787"; bind_set=1; shift ;;
    --ipv6) bind="[::]:8787"; bind_set=1; shift ;;
    --bind) bind="${2:?--bind requires HOST:PORT}"; bind_set=1; shift 2 ;;
    --state-db) state_db="${2:?--state-db requires PATH}"; state_db_set=1; shift 2 ;;
    --state-db-require-encryption) state_db_require_encryption="${2:?--state-db-require-encryption requires true|false}"; state_db_require_encryption_set=1; shift 2 ;;
    --state-file) state_file="${2:?--state-file requires PATH}"; state_file_set=1; shift 2 ;;
    --peer-id) peer_id="${2:?--peer-id requires ID}"; peer_id_set=1; shift 2 ;;
    --control-token) control_token="${2:?--control-token requires TOKEN}"; shift 2 ;;
    --control-token-file) control_token_file="${2:?--control-token-file requires PATH}"; shift 2 ;;
    --cors-allow-origin) cors_allow_origin="${2:?--cors-allow-origin requires ORIGIN}"; shift 2 ;;
    --sync-peer) sync_peer="${2:?--sync-peer requires URL[,URL]}"; shift 2 ;;
    --sync-peer-token) sync_peer_token="${2:?--sync-peer-token requires TOKEN}"; shift 2 ;;
    --sync-peer-token-file) sync_peer_token_file="${2:?--sync-peer-token-file requires PATH}"; shift 2 ;;
    --sync-interval-seconds) sync_interval_seconds="${2:?--sync-interval-seconds requires N}"; shift 2 ;;
    --sync-max-backoff-seconds) sync_max_backoff_seconds="${2:?--sync-max-backoff-seconds requires N}"; shift 2 ;;
    --dht-replication-factor) dht_replication_factor="${2:?--dht-replication-factor requires N}"; dht_replication_factor_set=1; shift 2 ;;
    --dht-routing-refresh-limit) dht_routing_refresh_limit="${2:?--dht-routing-refresh-limit requires N}"; dht_routing_refresh_limit_set=1; shift 2 ;;
    --dht-routing-refresh-max-targets) dht_routing_refresh_max_targets="${2:?--dht-routing-refresh-max-targets requires N}"; dht_routing_refresh_max_targets_set=1; shift 2 ;;
    --dht-transport) dht_transport="${2:?--dht-transport requires http-control|libp2p}"; dht_transport_set=1; shift 2 ;;
    --dht-peer-quarantine-consecutive-failures) dht_peer_quarantine_consecutive_failures="${2:?--dht-peer-quarantine-consecutive-failures requires N}"; dht_peer_quarantine_consecutive_failures_set=1; shift 2 ;;
    --rate-limit-window-seconds) rate_limit_window_seconds="${2:?--rate-limit-window-seconds requires N}"; rate_limit_window_seconds_set=1; shift 2 ;;
    --rate-limit-max-requests) rate_limit_max_requests="${2:?--rate-limit-max-requests requires N}"; rate_limit_max_requests_set=1; shift 2 ;;
    --log-format) log_format="${2:?--log-format requires text|json}"; log_format_set=1; shift 2 ;;
    --check-config) check_config=1; shift ;;
    --debug|--release)
      echo "run.sh 是 PRD runtime，不接受 $1；固定 build --release 并执行 target/release/lm_node" >&2
      exit 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown node option: $1" >&2; usage >&2; exit 2 ;;
  esac
done


is_loopback_bind() {
  local value="$1"
  case "$value" in
    127.*:*|localhost:*|\[::1\]:*) return 0 ;;
    *) return 1 ;;
  esac
}

validate_prd_run_security() {
  if [[ -z "$config_file" ]]; then
    if ! is_loopback_bind "$bind" && [[ -z "$control_token" && -z "$control_token_file" ]]; then
      cat >&2 <<'ERR'
拒绝启动：生产脚本绑定到非 loopback 地址时必须配置 --control-token 或 --control-token-file。

安全选项：
  - 仅本机使用：加 --local
  - 局域网/公网使用：加 --control-token-file /path/to/token，并确保文件 chmod 600
  - 复杂部署：使用 --config-file，并在配置中设置 control_token_file，置于 TLS 反向代理后
ERR
      exit 2
    fi
  fi
}

validate_prd_run_security

mkdir -p "$(dirname "$state_file")"
mkdir -p "$(dirname "$state_db")"
echo "启动 LM Talk 同步服务（PRD）"
if [[ -n "$config_file" ]]; then
  echo "配置文件：$config_file"
fi
if [[ -z "$config_file" || "$bind_set" == "1" ]]; then
  echo "绑定地址：$bind"
fi
if [[ -z "$config_file" || "$peer_id_set" == "1" ]]; then
  echo "Peer ID：$peer_id"
fi
if [[ -z "$config_file" || "$state_db_set" == "1" ]]; then
  echo "状态数据库：$state_db"
fi
if [[ "$state_db_require_encryption" == "true" ]]; then
  echo "状态数据库加密要求：required（当前 lm_node plain SQLite 会拒绝启动）"
fi
if [[ -z "$config_file" || "$state_db_require_encryption_set" == "1" ]]; then
  args+=(--state-db-require-encryption "$state_db_require_encryption")
fi
if [[ "$state_file_set" == "1" ]]; then
  echo "兼容 JSON 状态文件：$state_file"
fi
if [[ -n "$control_token" || -n "$control_token_file" ]]; then
  echo "控制面认证：Bearer token enabled"
else
  echo "控制面认证：未配置 token，仅允许 loopback 非 health 请求"
fi
if [[ -n "$cors_allow_origin" ]]; then
  echo "CORS allow origin：$cors_allow_origin"
fi
if [[ "$rate_limit_window_seconds" != "0" && "$rate_limit_max_requests" != "0" ]]; then
  echo "控制面限流：${rate_limit_max_requests} requests / ${rate_limit_window_seconds}s per client"
else
  echo "控制面限流：disabled"
fi
echo "日志格式：$log_format"
echo "DHT runner：transport=$dht_transport replication_factor=$dht_replication_factor refresh_limit=$dht_routing_refresh_limit refresh_max_targets=$dht_routing_refresh_max_targets quarantine_failures=$dht_peer_quarantine_consecutive_failures"
if [[ -n "$sync_peer" && "$sync_interval_seconds" != "0" ]]; then
  echo "自动同步：$sync_peer every ${sync_interval_seconds}s, max backoff ${sync_max_backoff_seconds}s"
  if [[ -n "$sync_peer_token" || -n "$sync_peer_token_file" ]]; then
    echo "同步认证：Bearer token enabled"
  fi
fi
if [[ "$check_config" == "1" ]]; then
  echo "配置检查：OK"
  exit 0
fi

echo "构建：release binary"
print_urls "$bind"
echo
echo "提示：PRD 禁止 cargo run，只执行 target/release/lm_node。公网部署请放在 TLS 反向代理后。"
echo

cd "$ROOT"
cargo build --release -p lm_node
args=(serve-control)
if [[ -n "$config_file" ]]; then
  args+=(--config-file "$config_file")
fi
if [[ -z "$config_file" || "$bind_set" == "1" ]]; then
  args+=(--bind "$bind")
fi
if [[ -z "$config_file" || "$peer_id_set" == "1" ]]; then
  args+=(--peer-id "$peer_id")
fi
if [[ -z "$config_file" || "$state_db_set" == "1" ]]; then
  args+=(--state-db "$state_db")
fi
if [[ -z "$config_file" || "$state_db_require_encryption_set" == "1" ]]; then
  args+=(--state-db-require-encryption "$state_db_require_encryption")
fi
if [[ "$state_file_set" == "1" ]]; then
  args+=(--state-file "$state_file")
fi
if [[ -n "$control_token" ]]; then
  args+=(--control-token "$control_token")
fi
if [[ -n "$control_token_file" ]]; then
  args+=(--control-token-file "$control_token_file")
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
if [[ -n "$sync_peer_token_file" ]]; then
  args+=(--sync-peer-token-file "$sync_peer_token_file")
fi
if [[ "$sync_interval_seconds" != "0" ]]; then
  args+=(--sync-interval-seconds "$sync_interval_seconds")
  args+=(--sync-max-backoff-seconds "$sync_max_backoff_seconds")
fi
if [[ -z "$config_file" || "$dht_replication_factor_set" == "1" ]]; then
  args+=(--dht-replication-factor "$dht_replication_factor")
fi
if [[ -z "$config_file" || "$dht_routing_refresh_limit_set" == "1" ]]; then
  args+=(--dht-routing-refresh-limit "$dht_routing_refresh_limit")
fi
if [[ -z "$config_file" || "$dht_routing_refresh_max_targets_set" == "1" ]]; then
  args+=(--dht-routing-refresh-max-targets "$dht_routing_refresh_max_targets")
fi
if [[ -z "$config_file" || "$dht_transport_set" == "1" ]]; then
  args+=(--dht-transport "$dht_transport")
fi
if [[ -z "$config_file" || "$dht_peer_quarantine_consecutive_failures_set" == "1" ]]; then
  args+=(--dht-peer-quarantine-consecutive-failures "$dht_peer_quarantine_consecutive_failures")
fi
if [[ -z "$config_file" || "$rate_limit_window_seconds_set" == "1" ]]; then
  args+=(--rate-limit-window-seconds "$rate_limit_window_seconds")
fi
if [[ -z "$config_file" || "$rate_limit_max_requests_set" == "1" ]]; then
  args+=(--rate-limit-max-requests "$rate_limit_max_requests")
fi
if [[ -z "$config_file" || "$log_format_set" == "1" ]]; then
  args+=(--log-format "$log_format")
fi
# Web admin panel: prefer a packaged node_admin.zip next to the binary,
# otherwise fall back to the built dist directory during development.
if [[ -f "$ROOT/node_admin.zip" ]]; then
  args+=(--web-admin "$ROOT/node_admin.zip")
elif [[ -f "$ROOT/target/release/node_admin.zip" ]]; then
  args+=(--web-admin "$ROOT/target/release/node_admin.zip")
elif [[ -d "$ROOT/apps/node-admin/dist" ]]; then
  args+=(--web-admin "$ROOT/apps/node-admin/dist")
fi
exec "$ROOT/target/release/lm_node" "${args[@]}"
