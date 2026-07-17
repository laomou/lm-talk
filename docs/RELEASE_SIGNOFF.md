# Release Sign-off / 发布签核

Use this document for the final go/no-go decision of a production release candidate. It complements `docs/RELEASE_EVIDENCE.md`, `docs/PRODUCTION_READINESS.md`, `docs/PROTOCOL_STABILITY.md`, and `docs/SECURITY_AUDIT_SCOPE.md`.

## Release candidate

- Version/tag:
- Commit SHA:
- Release branch:
- Date/time UTC:
- Release owner:
- Security reviewer:
- Operations reviewer:

## Required evidence links

| Evidence | Link / artifact | Reviewer | Status |
| --- | --- | --- | --- |
| Completed `docs/RELEASE_EVIDENCE.md` copy |  |  |  |
| `./scripts/release-check.sh full` output |  |  |  |
| Dependency audit / dependency review |  |  |  |
| SQLCipher smoke workflow artifact |  |  |  |
| SQLCipher release binary smoke artifact |  |  |  |
| Federation `run-all.sh` report |  |  |  |
| Federation chaos/load reports |  |  |  |
| Fuzz smoke report |  |  |  |
| Long fuzz campaign report/corpus/crash triage |  |  |  |
| External security audit report |  |  |  |
| Audit remediation commits |  |  |  |
| Public deployment topology and configs |  |  |  |
| macOS notarization evidence |  |  |  |
| Windows code signing evidence |  |  |  |
| SECURITY.md review |  |  |  |
| Completed `docs/RELEASE_RISK_REGISTER.md` review |  |  |  |

## Protocol freeze sign-off

| Item | Evidence | Status |
| --- | --- | --- |
| Stable object test vectors pass |  |  |
| DHT record kinds/namespaces frozen |  |  |
| Mailbox kind mapping frozen |  |  |
| ContactCard/DeviceCert merge policy tested |  |  |
| PreKey rotation/consumption interop tested |  |  |
| Error text/code dependencies reviewed |  |  |
| Deprecation/fallback policy accepted |  |  |

## Security sign-off

| Item | Decision / notes | Status |
| --- | --- | --- |
| Critical findings remediated |  |  |
| High findings remediated or explicitly accepted |  |  |
| Medium/low findings triaged |  |  |
| Risk register reviewed: no unowned Critical/High risks |  |  |
| Known metadata leakage accepted |  |  |
| Strict E2EE downgrade/fallback policy accepted |  |  |
| SQLCipher deployment evidence accepted |  |  |
| Token/CORS/deployment guidance accepted |  |  |

## Operations sign-off

| Item | Evidence / notes | Status |
| --- | --- | --- |
| Public bootstrap/federation topology validated |  |  |
| Backup/restore drill completed |  |  |
| Monitoring endpoints archived |  |  |
| Incident/contact process current |  |  |
| Rollback plan documented |  |  |
| Artifact checksums verified |  |  |

## Known limitations for this release

List limitations that are explicitly accepted for this release. Do not list blockers as accepted limitations unless they have an owner and mitigation.

- 

## Go / no-go decision

- Decision: `GO` / `NO-GO`
- Required follow-up issues:
- Approver names:
- Approval date/time UTC:

A production release is **NO-GO** if any of the following are missing:

1. External audit report and remediation review.
2. Long-running fuzz campaign report and crash triage.
3. Federation chaos/load evidence from realistic topology.
4. SQLCipher release artifact smoke evidence when state_db is used.
5. macOS notarization / Windows signing evidence for production desktop/native distribution.
6. Completed release evidence index.
7. Completed risk register with no unowned open Critical/High risks.
