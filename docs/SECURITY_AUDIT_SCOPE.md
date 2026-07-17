# Security Audit Scope / 安全审计范围

This document defines the security-review scope required before LM Talk can be called production-ready. It is a preparation checklist for an external auditor and for internal release review.

For an auditor-facing index of all review inputs, commands, and open blockers, start with `docs/EXTERNAL_AUDIT_PACKET.md`.

## Security goals

LM Talk aims to provide:

1. End-to-end encrypted direct messages, file packages, and group messages.
2. Decentralized discovery and delivery through DHT, Mailbox, and public node federation.
3. Multi-device delivery using signed device certificates, revocation, and per-device sealed slots.
4. Local state protection for Web IndexedDB and native node state databases/files.
5. Fail-closed behavior for strict E2EE policies and configured encrypted persistence.

## In scope

### Core cryptography (`crates/lm_core`)

Review:

- Identity creation, backup export/import, passphrase normalization, and storage-key derivation.
- Contact Card signing, verification, fingerprinting, and device certificate validation.
- Friend request/response signing and expiry handling.
- DirectEnvelope static X25519 + HKDF + XChaCha20-Poly1305 legacy path.
- X3DH PreKey bundle and signed one-time-prekey handling.
- Double Ratchet state transitions, skipped-message-key limits, replay/out-of-order handling, and state export/import.
- Group Sender Key distribution, group event policy, and sender-key rotation triggers.
- File package encryption, filename safety policy, size limits, and tamper handling.
- Message receipt signing and binding to message/conversation/delivery IDs.
- Serialization prefixes, canonical signed fields, object version checks, expiry checks, and maximum-size enforcement.

### WASM/Web bindings (`crates/lm_wasm`, `apps/web`)

Review:

- WASM API boundaries and error handling for all cryptographic helpers.
- Browser identity backup path and Web RNG usage.
- IndexedDB application-layer encryption, key rotation on passphrase re-encryption, deletion of identity-scoped tables, and migration failure behavior.
- Full data backup encryption/import merge/overwrite behavior.
- Local safety policy enforcement: verified-contact send/receive, sealed-slot send/receive, text/file filtering.
- Per-device envelope v1 format, sender signature, sealed slot encryption/opening, fallback/placeholder downgrade controls, and diagnostics.
- Self-sync package/request signing, sequence/gap repair, cached package replay/dedupe, own-device certificate sync, receipt-state sync, and outbox-summary sync.
- Contact Card update fanout/ACK/stale retry and DHT auto-refresh behavior, including the strict-E2EE repair-control exception for ContactCard/device-cert updates and device revocations.
- UI flows for fingerprint verification, QR scanning, device revocation, strict E2EE preflight, fail-closed content paths, and downgrade warnings.
- Web app background behavior after PWA removal: no Service Worker/background-sync key access; notifications are explicit foreground/user-driven flows unless a future reviewed implementation reintroduces background support.

### Native node (`crates/lm_node`)

Review:

- HTTP control plane request parsing, body/header limits, CORS, bearer-token checks, previous-token rotation, and rate limits.
- Mailbox store semantics: push/take/ack, duplicate/tombstone handling, TTL, quotas, per-sender/global rate limits, pagination, and crash recovery.
- PreKey publish/get/consume behavior and one-time-prekey consumed-state persistence.
- DHT record validation for PublicPeer, PreKey, MailboxHint, and ContactCard.
- Kademlia distance/query logic, closer-node filtering, peer health/quarantine, poisoned-record rejection, and replication/routing-refresh plans.
- Snapshot import/export merge semantics and sync-peer token handling.
- SQLite `state_db` schema, WAL/synchronous/busy-timeout/FK pragmas, permissions, SQLCipher provider, fail-closed encrypted persistence, wrong-passphrase rejection, and metrics.
- JSON `state_file` encryption, passphrase-file permissions, migration/fail-closed behavior, and compatibility risks.
- OpenMetrics output correctness and absence of sensitive payload leakage.
- CLI helpers used by release/federation smoke tests.

### Deployment and release artifacts

Review:

- `deploy/lm-node-public` and `deploy/lm-node-federation` templates, Caddy TLS proxy config, secret mounts, CORS origins, exposed ports, and encrypted-volume assumptions.
- `release-node.yml` artifact build matrix, SQLCipher Linux artifact, release notes, checksum generation, and `RELEASE_INFO.txt` feature stamping.
- Manual SQLCipher smoke and federation smoke/chaos/load report artifacts.
- macOS notarization and Windows code signing gap.

## Threat model focus

Auditors should consider at least:

- Malicious relay/mailbox/DHT nodes.
- Malicious or compromised contacts.
- Compromised/stolen devices and device revocation races.
- Replay, reorder, duplicate, delayed, and malformed network objects.
- Downgrade from sealed per-device slots to legacy/fallback envelopes.
- Poisoned DHT records and malicious closer-node responses.
- Browser local compromise short of live key extraction.
- Native node disk compromise with and without SQLCipher/external disk encryption.
- Token leakage or weak CORS/control-plane deployment.
- Resource exhaustion via mailbox/DHT/control-plane requests.

## Out of scope / not yet guaranteed

The following remain release blockers or explicit limitations unless separate evidence proves otherwise:

- External security audit completion and remediation.
- macOS notarization and Windows code signing.
- Long-running fuzz campaigns with triaged crashes and persisted corpus.
- Long-running public-network chaos/load tests across real hosted nodes.
- Complete metadata privacy; LM Talk protects content but still exposes operational metadata such as user IDs, timing, DHT keys, and mailbox delivery patterns.
- Full anonymity or traffic-analysis resistance.

## Required evidence for production claim

Before production readiness is claimed, collect and link:

- Completed `docs/RELEASE_EVIDENCE.md` for the exact release.
- `./scripts/release-check.sh full` output.
- SQLCipher smoke/deploy artifacts showing encrypted state DB metrics and wrong-passphrase failure.
- Federation `run-all.sh` report and public deployment topology.
- Fuzz campaign reports with corpus/crash artifacts and triage notes.
- External audit report and remediation commits.
- Dependency-audit/dependency-review evidence.
- Signing/notarization evidence for distributed desktop/native artifacts.

## Auditor deliverables

Expected auditor output:

1. Findings with severity, exploit narrative, affected files/protocol objects, and reproduction steps.
2. Recommended fixes and verification method.
3. Confirmation of reviewed commit SHA and configuration assumptions.
4. Explicit statement of unresolved risks and accepted limitations.
