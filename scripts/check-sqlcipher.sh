#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== SQLCipher provider initializes cipher =="
cargo test -p lm_node --bin lm_node --features sqlcipher sqlcipher_provider_initializes_cipher

echo "== SQLCipher rejects wrong passphrase =="
cargo test -p lm_node --bin lm_node --features sqlcipher sqlcipher_state_db_rejects_wrong_passphrase

echo "== SQLCipher feature compiles provider boundary =="
cargo test -p lm_node --bin lm_node --features sqlcipher state_db_encryption_provider_models_current_modes

echo "== sqlcipher smoke ok =="
