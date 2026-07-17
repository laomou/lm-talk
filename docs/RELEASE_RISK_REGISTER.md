# Release Risk Register / 发布风险登记

Use this register to track residual risks that remain after implementation, testing, and audit remediation. A production release must explicitly accept or resolve every non-low residual risk.

## Status values

- `Open`: risk exists and needs mitigation or acceptance.
- `Mitigated`: mitigation implemented and evidence linked.
- `Accepted`: release owner and security reviewer accept the residual risk.
- `Rejected`: risk is not accepted; release is no-go until fixed.
- `Closed`: risk no longer applies.

## Risk register

| ID | Risk | Severity | Status | Mitigation / evidence | Owner | Release decision |
| --- | --- | --- | --- | --- | --- | --- |
| RISK-001 | LM Talk protects content but does not provide anonymity or traffic-analysis resistance; user IDs, timing, DHT keys, record kinds, and message sizes may be observable. | Medium | Open | Documented in `docs/CRYPTO_REVIEW_NOTES.md`; public messaging must avoid anonymity claims. |  |  |
| RISK-002 | Public node operators can observe metadata and may degrade availability even though they cannot decrypt E2EE content. | Medium | Open | Federation, multi-node failover, strict E2EE, and public deployment runbook; needs real public topology evidence. |  |  |
| RISK-003 | Legacy DirectEnvelope and placeholder per-device slots remain as compatibility paths and may be weaker than strict sealed-slot mode. | Medium | Open | Strict E2EE policy blocks fallback; UI warnings and preflight exist; deprecation plan required before production defaults. |  |  |
| RISK-004 | Browser/local device compromise can expose live decrypted data or keys while user is logged in. | High | Open | IndexedDB encryption and local deletion exist; cannot defend against fully compromised runtime. Requires explicit user guidance and audit review. |  |  |
| RISK-005 | macOS notarization and Windows code signing are not implemented for production-trust native distribution. | High | Open | Release checklist marks this as required before production distribution. |  |  |
| RISK-006 | Long-running fuzz, chaos, and load evidence has scripts/templates but not necessarily completed production-duration runs. | High | Open | `scripts/fuzz-campaign.sh`, federation runbooks, and evidence templates exist; release must archive real reports. |  |  |
| RISK-007 | External security audit has scope/tracker but no completed third-party report in repository evidence. | High | Open | `docs/SECURITY_AUDIT_SCOPE.md` and `docs/AUDIT_REMEDIATION_TRACKER.md`; release is no-go without audit report/remediation. |  |  |
| RISK-008 | SQLCipher security depends on building and deploying the correct SQLCipher artifact with strong passphrase handling. | Medium | Open | SQLCipher feature, deploy smoke, release smoke artifact, and evidence checklist; release must archive exact artifact proof. |  |  |
| RISK-009 | Dependency advisory exceptions may become reachable if features change or upstream dependency behavior changes. | Medium | Open | `scripts/audit.sh` documents narrow ignores; revisit on dependency upgrades and feature changes. |  |  |
| RISK-010 | Public federation availability depends on operator deployment quality, TLS/CORS correctness, token hygiene, and backup operations. | Medium | Open | Deployment templates and runbooks exist; production evidence requires real public deployment report. |  |  |

## Acceptance rules

- Critical risks cannot be accepted for a production release.
- High risks require written acceptance by release owner and security reviewer, plus mitigation and user/operator communication.
- Medium risks require documented mitigation or explicit release-note disclosure.
- Accepted risks must be copied into `docs/RELEASE_SIGNOFF.md` for the release candidate.

## Machine gate

Run the production risk gate before release sign-off:

```bash
./scripts/risk-register-gate.sh
```

Strict mode exits non-zero while any Medium/High/Critical risk is `Open` or `Rejected`, lacks an owner, or lacks a release decision. To print the same findings without failing a larger evidence collection job:

```bash
RISK_REGISTER_GATE_MODE=report ./scripts/risk-register-gate.sh
```

A production release must not override this gate; resolve, mitigate, or explicitly accept every non-low residual risk according to the acceptance rules above.

## Review checklist

Before release sign-off:

- [ ] Every `Open` high/medium risk has an owner.
- [ ] Every accepted risk has evidence and release-note wording.
- [ ] Every mitigated risk links the fixing commit/test/artifact.
- [ ] No critical risk remains open or accepted.
- [ ] This register is linked from `docs/RELEASE_EVIDENCE.md`.
