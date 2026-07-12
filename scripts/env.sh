#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'USAGE'
Usage: ./scripts/env.sh [all|web|wasm]

Prepare local toolchain helpers. This is setup only; it does not build or run LM Talk.

Targets:
  all   install web/node helpers and wasm target/tools (default)
  web   install project-local Node.js and npm dependencies
  wasm  install wasm32 target and wasm-pack
USAGE
}

target="${1:-all}"
case "$target" in
  all|web|wasm) ;;
  -h|--help|help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac

setup_web() {
  mkdir -p "$ROOT/.tools"
  cd "$ROOT/.tools"
  if [[ ! -d node ]]; then
    echo "Downloading Node.js latest v22.x..."
    curl -fsSL https://nodejs.org/dist/latest-v22.x/SHASUMS256.txt -o SHASUMS256.txt
    tarball="$(grep 'linux-x64.tar.xz$' SHASUMS256.txt | awk '{print $2}' | head -n1)"
    curl -fsSLO "https://nodejs.org/dist/latest-v22.x/$tarball"
    grep " $tarball$" SHASUMS256.txt | sha256sum -c -
    tar -xf "$tarball"
    rm -rf node
    mv "${tarball%.tar.xz}" node
  fi
  export PATH="$ROOT/.tools/node/bin:$PATH"
  node --version
  npm --version
  cd "$ROOT/apps/web"
  npm install
}

setup_wasm() {
  rustup target add wasm32-unknown-unknown
  if ! command -v wasm-pack >/dev/null 2>&1; then
    cargo install wasm-pack
  fi
}

case "$target" in
  all) setup_web; setup_wasm ;;
  web) setup_web ;;
  wasm) setup_wasm ;;
esac
