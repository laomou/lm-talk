# Security Policy

LM Talk is an end-to-end encrypted, local-first IM project. Security reports are welcome, but the project should **not** be treated as production-ready until the release checklist blockers are closed.

## Supported versions

This repository is pre-1.0. Only the current `main`/`master` branch is in scope for security fixes unless a release branch is explicitly announced.

## How to report a vulnerability

Please report security issues privately. Do **not** open a public issue containing exploit details, keys, tokens, backups, message plaintext, or other sensitive material.

Include as much of the following as possible:

- Affected component: `lm_core`, `lm_wasm`, Web UI/PWA, `lm_node`, deployment scripts, or docs.
- Impact and attacker capability required.
- Reproduction steps against a minimal local setup.
- Whether the issue exposes plaintext, identity seed material, backup text, bearer tokens, local IndexedDB data, node `state_db`, DHT records, or mailbox metadata.
- Relevant commit hash and platform/browser/OS.

If you need to include logs, first remove or redact:

- identity backups and data backups;
- passphrases, seed material, private keys, and device private material;
- bearer tokens and `http://host|token` sync URLs;
- message plaintext, attachment contents, and complete protocol payloads unless strictly necessary.

## Expected response

For private reports, maintainers should acknowledge receipt, triage severity, and coordinate a fix before public disclosure. If maintainers are unavailable, avoid publishing working exploit details until users have had reasonable time to update.

## Known non-production blockers

The following are known blockers and do not need to be reported as new vulnerabilities unless you have a concrete exploit or bypass:

- No completed external security audit.
- No completed long-running fuzz campaign with saved corpus and crash triage.
- Native node `state_db` is plain SQLite, not SQLCipher/encrypted. The node exposes `state_db_encrypted=false`; `state_db_require_encryption=true` intentionally fails closed in this build.
- DHT/Kademlia is still a hardened scaffold, not a fully production-grade public DHT with Sybil resistance, peer scoring, alpha concurrency, and public deployment model.
- Relay/TURN replacement and public relay abuse controls are not production-complete.
- Web/PWA background events do not decrypt or sync messages in the Service Worker; users must open the app to use local keys.

## Security design references

- `docs/security/SECURITY_MODEL.md` — goals, non-goals, and threat boundaries.
- `docs/release/RELEASE_CHECKLIST.md` — release candidate gate and remaining manual blockers.
- `docs/testing/FUZZING.md` — fuzz harnesses and campaign requirements.
- `docs/deploy/NODE_CONFIG.md` — node deployment and operational hardening guidance.
