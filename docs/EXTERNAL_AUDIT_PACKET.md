# External Audit Packet / 外部安全审计交付包

This packet is the auditor-facing entry point for reviewing LM Talk as a decentralized, end-to-end encrypted instant messaging system. It does **not** claim production readiness by itself; it indexes the exact areas, evidence, and open blockers that an external audit must validate before a production release.

## Audit target

- Product goal: decentralized E2EE instant messaging with Web/WASM clients and native public/federation nodes.
- Primary security goals: message/file/group content confidentiality, signed identities/contact cards/devices, strict E2EE downgrade controls, encrypted local persistence, and malicious-node resilience.
- Current distribution target: tag-based native `lm_node` artifacts for Linux, macOS, and Windows plus Web static build evidence.

## Required commit and release identifiers

Fill these before sending the packet to an auditor:

| Field | Value |
| --- | --- |
| Audit commit SHA | `TODO` |
| Release candidate tag | `TODO` |
| Evidence directory or CI run | `TODO` |
| Auditor / firm | `TODO` |
| Audit start date | `TODO` |
| Audit owner | `TODO` |

## Start here

1. `docs/SECURITY_AUDIT_SCOPE.md` — definitive in-scope areas and required auditor deliverables.
2. `docs/SECURITY_MODEL.md` — concise security goals, non-goals, and product boundaries.
3. `docs/CRYPTO_REVIEW_NOTES.md` — protocol/crypto design notes and known metadata limitations.
4. `docs/PROTOCOL_STABILITY.md` — protocol freeze status and compatibility rules.
5. `docs/TEST_VECTOR_COVERAGE.md` — canonical vector coverage for signed/encrypted wire objects.
6. `docs/RELEASE_RISK_REGISTER.md` — residual risks that must be mitigated, accepted, or marked no-go.
7. `docs/AUDIT_REMEDIATION_TRACKER.md` — finding/remediation tracker to update during the audit.
8. `docs/RELEASE_EVIDENCE.md` and `docs/RELEASE_SIGNOFF.md` — release evidence/signoff templates for the audited release.

## Code review map

| Area | Paths | Review focus |
| --- | --- | --- |
| Core crypto/protocol | `crates/lm_core/src`, `test-vectors/`, `crates/lm_core/tests` | Identity, ContactCard, device cert/revoke, X3DH, Double Ratchet, group sender keys, file packages, receipts, expiry/version/size checks. |
| WASM bridge | `crates/lm_wasm/src` | JS/WASM boundary validation, error mapping, crypto helper exposure, per-device slot sealing/opening. |
| Web client | `apps/web/src`, `apps/web/tests` | Strict E2EE preflight, fingerprint/device UX, IndexedDB encryption, backup/import, self-sync, sealed-slot downgrade blocking, local deletion/re-encryption. |
| Native node | `crates/lm_node/src` | Control-plane parsing/auth/rate limits, Mailbox/PreKey/DHT behavior, peer quarantine, SQLCipher provider, metrics, persistence recovery. |
| Federation deployment | `deploy/lm-node-public`, `deploy/lm-node-federation` | TLS/Caddy templates, secret handling, node topology, smoke/chaos/load scripts, operator guidance. |
| Release supply chain | `.github/workflows`, `scripts/package-node-release.py`, `scripts/verify-node-release.sh`, `scripts/preprod-evidence.sh` | Cross-platform builds, checksums, SQLCipher artifact proof, evidence collection, dependency audit gates. |

## Test and evidence commands

Auditors should run or review archived output for the exact audited commit/tag:

```bash
./scripts/release-check.sh full
./scripts/audit.sh
./scripts/risk-register-gate.sh
FUZZ_SMOKE_REPORT=fuzz-smoke-report.json ./scripts/fuzz-smoke.sh
FUZZ_CAMPAIGN_DURATION=3600 ./scripts/fuzz-campaign.sh
./scripts/sqlcipher-smoke.sh
./scripts/sqlcipher-deploy-smoke.sh
RUN_RELEASE_ASSET_VERIFY=1 RELEASE_TAG_VERIFY=<tag> RELEASE_VERSION=<tag> ./scripts/preprod-evidence.sh
```

For federation evidence:

```bash
cd deploy/lm-node-federation
./run-all.sh
./chaos-smoke.sh
MESSAGE_COUNT=100 ./load-smoke.sh
```

Long-running production-readiness campaigns should increase fuzz, chaos, and load durations beyond smoke defaults and archive corpus/crash directories plus JSON reports.

## High-priority audit questions

- Can any untrusted node, DHT peer, Mailbox operator, or relay observe or modify message plaintext?
- Are strict sealed per-device slots enforced without silent fallback when strict E2EE is enabled?
- Are ContactCards, device certificates, revocations, PreKeys, receipts, and DHT records bound to the intended identity/key/version/expiry?
- Are replay, reorder, duplicate, delayed, oversized, malformed, and expired objects rejected or handled safely?
- Does self-sync preserve trust/device/revocation state without resurrecting stale or revoked devices?
- Do native node quotas/rate limits prevent practical unauthenticated or token-authenticated resource exhaustion?
- Does SQLCipher mode fail closed and report reliable encrypted-state metrics for the exact release artifact?
- Do release artifacts and dependency audit exceptions avoid reachable supply-chain risk?

## Known release blockers to verify

The following are expected to remain no-go until external evidence is attached:

- Completed third-party audit report and remediation verification.
- Long-running fuzz campaign artifacts and triage notes.
- Long-running public federation chaos/load report with real topology.
- macOS notarization and Windows code-signing evidence for production-trust native distribution.
- Completed `docs/RELEASE_SIGNOFF.md` and resolved/accepted entries in `docs/RELEASE_RISK_REGISTER.md`.

## Audit output requirements

Auditor findings should include:

1. Severity, affected component/path, and exploit narrative.
2. Reproduction steps, payloads, or test vectors where applicable.
3. Recommended fix and verification command/test.
4. Reviewed commit SHA and release tag.
5. Explicit residual-risk statement for any accepted limitation.

Track findings in `docs/AUDIT_REMEDIATION_TRACKER.md` and link fixing commits, tests, and release evidence before production signoff.
