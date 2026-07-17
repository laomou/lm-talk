# Audit Remediation Tracker / 审计修复跟踪

Use this tracker after each internal or external security audit. Every finding must have an owner, status, fix/mitigation evidence, and release decision before LM Talk can be claimed production-ready.

## Status values

- `Open`: finding accepted for tracking, no fix merged yet.
- `In progress`: fix or mitigation is being implemented.
- `Fixed`: fix merged and verified.
- `Accepted risk`: risk explicitly accepted for this release with mitigation/justification.
- `Duplicate`: covered by another finding.
- `Won't fix`: not applicable or rejected, with rationale.

## Severity values

- `Critical`: exploitable compromise of E2EE content, identity private keys, SQLCipher state, or remote code execution.
- `High`: serious bypass of trust, device revocation, strict E2EE, DHT poisoning protection, auth, or persistence confidentiality.
- `Medium`: meaningful security degradation with prerequisites or limited blast radius.
- `Low`: hardening, diagnostic leakage, or defense-in-depth issue.
- `Info`: non-security or documentation/process finding.

## Findings

| ID | Severity | Component | Summary | Status | Owner | Fix commit(s) | Verification evidence | Release decision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUDIT-0001 | TBD | TBD | Placeholder: replace with first audit finding. | Open |  |  |  |  |

## Required fields per finding

Each finding should include, either in the table or linked issue:

- Affected component(s): `lm_core`, `lm_wasm`, Web, `lm_node`, deployment, CI/release, docs.
- Threat model / exploit narrative.
- Reproduction steps or proof-of-concept when safe to store.
- Expected security property.
- Actual observed behavior.
- Fix plan and owner.
- Fix commit(s) or accepted-risk rationale.
- Verification command(s), test(s), or artifact(s).
- Reviewer sign-off.

## Release gate

A production release is **NO-GO** if any finding is:

- `Critical` and not `Fixed`.
- `High` and not `Fixed` or explicitly `Accepted risk` by the release owner and security reviewer.
- Missing verification evidence after being marked `Fixed`.

Before release sign-off, copy the final finding summary into `docs/RELEASE_SIGNOFF.md` and link the audit report in `docs/RELEASE_EVIDENCE.md`.

## Suggested issue labels

- `audit`
- `security`
- `severity:critical`
- `severity:high`
- `severity:medium`
- `component:core`
- `component:wasm`
- `component:web`
- `component:node`
- `component:deploy`
- `release-blocker`
