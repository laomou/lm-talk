#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== core e2e secure flow =="
cargo test -p lm_core --test e2e_secure_flow

echo "== node e2e prekey/sync/mailbox/ratchet flow =="
cargo test -p lm_node --test e2e_node_flow

echo "== e2e ok =="
