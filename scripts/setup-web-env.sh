#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT/.tools"
cd "$ROOT/.tools"
if [ ! -d node ]; then
  echo "Downloading Node.js latest v22.x..."
  curl -fsSL https://nodejs.org/dist/latest-v22.x/SHASUMS256.txt -o SHASUMS256.txt
  TARBALL="$(grep 'linux-x64.tar.xz$' SHASUMS256.txt | awk '{print $2}' | head -n1)"
  curl -fsSLO "https://nodejs.org/dist/latest-v22.x/$TARBALL"
  grep " $TARBALL$" SHASUMS256.txt | sha256sum -c -
  tar -xf "$TARBALL"
  rm -rf node
  mv "${TARBALL%.tar.xz}" node
fi
export PATH="$ROOT/.tools/node/bin:$PATH"
node --version
npm --version
rustup target add wasm32-unknown-unknown
if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack
fi
cd "$ROOT/apps/web"
npm install
