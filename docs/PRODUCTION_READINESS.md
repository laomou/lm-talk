# Production Readiness Dashboard / 生产就绪看板

This dashboard summarizes the remaining work before LM Talk can claim to be a production-ready decentralized end-to-end encrypted instant messaging system.

Status legend:

- **Done**: implemented and covered by automated tests or documented release evidence.
- **Evidence needed**: implementation exists, but production claim needs archived run/audit/deployment evidence.
- **Open**: implementation or operational process is incomplete.
- **Blocked for production**: cannot claim production readiness until resolved.

## Overall status

| Area | Status | Notes |
| --- | --- | --- |
| MVP/demo usability | Done | Core user flows, Web UI, Mailbox sync, DHT discovery, backups, and diagnostics are available. |
| End-to-end content encryption | Done | Direct messages, Ratchet path, files, groups, and sealed per-device slots are implemented. |
| Decentralized discovery/delivery | Done / Evidence needed | DHT ContactCard/PreKey/MailboxHint/PublicPeer and Mailbox federation exist; public multi-node deployment evidence still needed. |
| Multi-device E2EE | Done / Evidence needed | Device certs, revocation, self-sync, sealed slots, strict E2EE policy, and receipt/outbox summaries exist; long-running interop evidence still needed. |
| Native state DB encryption | Done / Evidence needed | SQLCipher feature, deploy smoke, release artifact smoke, and docs exist; release-specific artifact evidence must be archived. |
| Release evidence workflow | Done | `docs/RELEASE_EVIDENCE.md`, collection helper, SQLCipher smoke, federation reports, fuzz reports exist. |
| External security audit | Blocked for production | Audit scope exists; third-party audit report and remediation are still missing. |
| Long-running fuzz/chaos/load | Evidence needed | Scripts and report formats exist; real long-duration reports/corpus/crash triage are still missing. |
| Release signing/notarization | Open | macOS notarization and Windows code signing are not implemented. |
| Protocol freeze | Evidence needed | Stability policy and many vectors exist; freeze checklist still has open items. |

## Feature readiness

| Capability | Status | Evidence / next action |
| --- | --- | --- |
| Identity backup/restore | Done | `test-vectors/identity_v1.json`, `backup_v1.json`, core/WASM tests. |
| Contact Cards and fingerprints | Done | `contact_card_v1.json`, DHT ContactCard vector, QR/copy/scan UI. |
| Friend request/response | Done | `friend_request_v1.json`, Web e2e flow. |
| Direct message encryption | Done | `message_crypto_v1.json`, core tests. |
| X3DH / PreKey | Done | `prekey_v1.json`, DHT PreKey publish/find, one-time-prekey state. |
| Double Ratchet | Done / Evidence needed | `ratchet_v1.json`; external audit still required. |
| File encryption packages | Done / Evidence needed | Core/Web tests exist; stable vectors still medium-priority missing. |
| Group Sender Key / events | Done / Evidence needed | Functional tests exist; stable vectors and audit still needed. |
| Per-device sealed slots | Done / Evidence needed | Web sealed slot implementation and `per_device_envelope_v1.json`; external audit required. |
| Device cert/revoke | Done | `device_v1.json`; UI and fanout flows exist. |
| Strict E2EE policy | Done | Send/receive verified contact + sealed slot enforcement, preflight and blockers UI. |
| Full encrypted data backup | Done | Web backup export/import/merge and own-Mailbox backup flows. |
| Lightweight self-sync | Done | Signed packages, gap repair, cached packages, receipt state, outbox summary, own device cert sync. |

## Node / network readiness

| Capability | Status | Evidence / next action |
| --- | --- | --- |
| Mailbox push/take/ack | Done | Node unit/e2e tests, federation smoke/load scripts. |
| Mailbox quotas/rate limits | Done | Node tests and metrics. |
| DHT record validation | Done | PublicPeer/PreKey/MailboxHint/ContactCard validation tests. |
| DHT hardening | Done / Evidence needed | Poisoning/quarantine tests exist; public long-running network evidence missing. |
| Snapshot sync | Done | Node tests and federation scripts. |
| Public node deployment template | Done | `deploy/lm-node-public`. |
| Three-node federation template | Done | `deploy/lm-node-federation`. |
| Federation smoke/chaos/load | Done / Evidence needed | Scripts exist; archive real report artifacts for releases. |
| Relay/TURN replacement | Open | Need production connectivity strategy that does not create a hard central dependency. |
| Public bootstrap topology | Evidence needed | Templates exist; need real hosted topology report. |

## Persistence / local data readiness

| Capability | Status | Evidence / next action |
| --- | --- | --- |
| Web IndexedDB encryption | Done / Evidence needed | E2E tests cover encryption/key rotation; audit still needed. |
| Web identity-scoped deletion | Done | E2E tests. |
| Native SQLite state_db | Done | WAL/FULL/FK/busy-timeout/permissions tests. |
| SQLCipher state_db | Done / Evidence needed | `sqlcipher` feature, smoke scripts, release artifact smoke; archive release-specific proof. |
| JSON state_file encryption | Done | App-layer XChaCha20-Poly1305, fail-closed checks. |
| Backup/restore drills | Evidence needed | Feature exists; release should archive restore drill results. |

## Release evidence readiness

| Evidence | Status | Required artifact |
| --- | --- | --- |
| Quick CI gate | Done | CI `release-check` or `./scripts/release-check.sh quick`. |
| Full release gate | Evidence needed | `./scripts/release-check.sh full` output for release commit. |
| Dependency audit | Done / Evidence needed | CI `dependency-audit`; archive for release. |
| SQLCipher smoke | Done / Evidence needed | `scripts/sqlcipher-smoke.sh`, SQLCipher Smoke workflow artifact. |
| SQLCipher release binary smoke | Done / Evidence needed | `lm_node-linux-x86_64-sqlcipher-smoke` release workflow artifact. |
| Federation validation | Done / Evidence needed | `federation-report.json` from `run-all.sh` or workflow artifact. |
| Fuzz smoke | Done / Evidence needed | `FUZZ_SMOKE_REPORT=... ./scripts/fuzz-smoke.sh`. |
| Long fuzz campaign | Evidence needed | `fuzz-campaign-report.json`, corpus, artifacts, triage notes. |
| External security audit | Blocked for production | Audit report and remediation commits. |
| Signing/notarization | Open | macOS notarization and Windows signing evidence. |

## Go / no-go criteria

A production-ready claim is **NO-GO** until all of the following are true:

1. `docs/RELEASE_EVIDENCE.md` is filled for the exact release commit.
2. SQLCipher release artifact evidence proves `state_db_encrypted=true` for the selected deployment mode.
3. Long-running fuzz campaign reports are archived with crash triage.
4. Federation chaos/load reports are archived from a realistic topology.
5. External security audit is complete and critical/high findings are remediated or explicitly accepted.
6. Protocol freeze checklist in `docs/PROTOCOL_STABILITY.md` is satisfied.
7. macOS/Windows release trust requirements are met for any production desktop/native distribution.
8. SECURITY.md contact and vulnerability-handling process are verified for the release branch.

## Current recommendation

LM Talk is suitable for advanced demos, local federation testing, and pre-production security review. It should not yet be marketed as production-ready until the evidence and audit items above are complete.
