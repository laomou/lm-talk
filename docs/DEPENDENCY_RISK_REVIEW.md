# Dependency Risk Review / 依赖风险复核

This document tracks dependency-security exceptions and the process for deciding whether a vulnerable dependency is reachable in LM Talk. It complements `scripts/audit.sh`, CI `dependency-audit`, GitHub `dependency-review`, and `docs/RELEASE_RISK_REGISTER.md`.

## Current audit gates

| Ecosystem | Command / CI | Release evidence |
| --- | --- | --- |
| Rust | `./scripts/audit.sh` runs `cargo audit --deny warnings` | CI `dependency-audit` log or local output |
| Web npm | `npm audit --audit-level high` in `apps/web` | CI `dependency-audit` log or local output |
| PR dependency diff | GitHub `dependency-review` | PR check status |

`SKIP_CARGO_AUDIT=1` is only for environments without `cargo-audit`; it must not be used as release evidence.

## Current ignored Rust advisories

`scripts/audit.sh` currently ignores the following advisories. These exceptions must be revisited whenever dependencies or enabled features change.

| Advisory | Current rationale | Reachability assumption | Re-evaluate when | Release status |
| --- | --- | --- | --- | --- |
| `RUSTSEC-2026-0118` | Transitive `hickory-proto` advisory pulled by unused optional `libp2p` DNS/mDNS dependency metadata. | LM Talk enables libp2p TCP/noise/yamux/request-response only; DNS/mDNS features are not enabled. | `libp2p` upgraded, DNS/mDNS features enabled, or advisory scope changes. | Exception allowed only with documented CI audit output. |
| `RUSTSEC-2026-0119` | Same `hickory-proto` dependency family as above. | Same as above. | Same as above. | Exception allowed only with documented CI audit output. |
| `RUSTSEC-2024-0436` | `paste` warning via transitive Linux netlink/proc-macro path, currently not security-relevant to LM Talk runtime protocol. | No direct LM Talk protocol parsing, crypto, node control, or Web boundary depends on `paste` behavior. | netlink stack or dependent crates become part of exposed node control/data path; dependency upgraded or advisory changes. | Exception allowed only with documented CI audit output. |

## Review workflow for a new advisory

1. Identify direct or transitive dependency and enabled feature path.
2. Determine whether vulnerable code is reachable from:
   - Web/WASM boundary;
   - native node control plane;
   - Mailbox/DHT parsing;
   - cryptographic operations;
   - deployment/build/release tooling.
3. If reachable and exploitable, treat as a release blocker until fixed or mitigated.
4. If not reachable, document the feature/path reason in this file and add the narrowest possible `cargo audit --ignore` entry.
5. Add a follow-up item to revisit the exception on dependency upgrade.
6. Link the decision in `docs/RELEASE_RISK_REGISTER.md` when severity is medium or higher.

## Dependency update policy

- Prefer removing unused dependency features before adding audit exceptions.
- Keep `libp2p` features minimal: macros, noise, request-response, json, tcp, tokio, yamux.
- Keep SQLCipher feature explicit; do not enable it by default for all artifacts unless release policy changes.
- For Web dependencies, avoid adding runtime packages that execute untrusted HTML/markdown or broaden browser permission surface without review.
- Dependabot PRs should include CI `dependency-review` status and release-note impact when security relevant.

## Release evidence requirements

For each release candidate, archive:

- CI `dependency-audit` job log;
- `./scripts/audit.sh` output if run locally;
- list of active `cargo audit --ignore` exceptions from this file;
- PR `dependency-review` status for dependency-changing PRs;
- any accepted dependency risks copied into `docs/RELEASE_RISK_REGISTER.md`.

## No-go criteria

A production release is **NO-GO** if:

- A reachable high/critical advisory is unresolved.
- An audit exception lacks reachability rationale.
- `npm audit --audit-level high` fails without a documented accepted risk.
- `cargo audit --deny warnings` fails for an advisory not explicitly reviewed here.
