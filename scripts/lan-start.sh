#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$ROOT/.tools/node/bin:$PATH"

usage() {
  cat <<'USAGE'
Usage: ./scripts/lan-start.sh [options]

Build and start a LAN-ready LM Talk Web page and sync service on this machine.
The Web page is served as static production files. The sync service requires a
Bearer token and keeps its state in a local SQLite database.

Options:
  --bind HOST             listen address for both services (default: 0.0.0.0)
  --web-port PORT        Web page port (default: 4173)
  --node-port PORT       sync service port (default: 8787)
  --state-dir PATH       state and token directory (default: ./.lan)
  --token-file PATH      reuse or create this control token file
  --no-build             start existing production artifacts without rebuilding
  -h, --help             show this help

Example:
  ./scripts/lan-start.sh
  ./scripts/lan-start.sh --web-port 8080 --state-dir /srv/lm-talk
USAGE
}

bind="0.0.0.0"
web_port="4173"
node_port="8787"
state_dir="$ROOT/.lan"
token_file=""
build=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bind) bind="${2:?--bind requires HOST}"; shift 2 ;;
    --web-port) web_port="${2:?--web-port requires PORT}"; shift 2 ;;
    --node-port) node_port="${2:?--node-port requires PORT}"; shift 2 ;;
    --state-dir) state_dir="${2:?--state-dir requires PATH}"; shift 2 ;;
    --token-file) token_file="${2:?--token-file requires PATH}"; shift 2 ;;
    --no-build) build=0; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for command in python3 curl; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command is required to start the LAN deployment." >&2
    exit 1
  fi
done

mkdir -p "$state_dir"
state_dir="$(cd "$state_dir" && pwd)"
token_file="${token_file:-$state_dir/control.token}"

if [[ ! -s "$token_file" ]]; then
  umask 077
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 32 > "$token_file"
  else
    od -An -N32 -tx1 /dev/urandom | tr -d ' \n' > "$token_file"
  fi
fi
chmod 600 "$token_file"

lan_ip=""
if command -v hostname >/dev/null 2>&1; then
  lan_ip="$(hostname -I 2>/dev/null | tr ' ' '\n' | awk '/^[0-9]+\./ && $1 !~ /^127\./ { print; exit }')"
fi
if [[ -z "$lan_ip" ]]; then
  echo "Could not determine a LAN IPv4 address. Use the host IP shown by your system." >&2
  lan_ip="<this-machine-lan-ip>"
fi

web_origin="http://$lan_ip:$web_port"
node_url="http://$lan_ip:$node_port"
sync_address="$node_url|$(cat "$token_file")"

if [[ "$build" == "1" ]]; then
  echo "== Build sync service =="
  (cd "$ROOT" && cargo build --release -p lm_node)
  echo "== Build Web app =="
  (cd "$ROOT/apps/web" && npm run build)
fi

node_binary="$ROOT/target/release/lm_node"
web_dir="$ROOT/apps/web/dist"
if [[ ! -x "$node_binary" ]]; then
  echo "Missing $node_binary. Run without --no-build first." >&2
  exit 1
fi
if [[ ! -f "$web_dir/index.html" ]]; then
  echo "Missing $web_dir/index.html. Run without --no-build first." >&2
  exit 1
fi

"$node_binary" serve-control \
  --bind "$bind:$node_port" \
  --control-token-file "$token_file" \
  --state-db "$state_dir/lm-node.sqlite3" \
  --cors-allow-origin "$web_origin" &
node_pid=$!

cleanup() {
  kill "$node_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

for _ in {1..20}; do
  if curl --silent --fail "http://127.0.0.1:$node_port/health" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$node_pid" 2>/dev/null; then
    echo "Sync service failed to start." >&2
    exit 1
  fi
  sleep 0.25
done

echo
echo "LM Talk LAN deployment is running."
echo "Web page:        $web_origin"
echo "Sync service:    $node_url"
echo "Sync address:    $sync_address"
echo
echo "Open the Web page on each device, then go to:"
echo "我 → 同步与安全 → 编辑地址"
echo "Paste the Sync address above, save it, and enable sync."
echo
echo "Press Ctrl+C to stop both services."

exec python3 -m http.server "$web_port" --bind "$bind" --directory "$web_dir"
