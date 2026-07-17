# Release Evidence Index / 发布证据索引

Use this file as the evidence template for every release candidate. Copy it to a release-specific file or issue, fill in artifact links, and keep the completed copy with the release notes.

## Release candidate

- Version/tag:
- Commit SHA:
- Date/time UTC:
- Operator/reviewer:

## Required automated gates

| Gate | Required evidence | Artifact/link | Status |
| --- | --- | --- | --- |
| Quick release check | `./scripts/release-check.sh quick` or CI `release-check` log |  |  |
| Full release check | `./scripts/release-check.sh full` output |  |  |
| Dependency audit | `./scripts/audit.sh` / CI `dependency-audit` log |  |  |
| SQLCipher smoke | `./scripts/sqlcipher-smoke.sh` or SQLCipher Smoke workflow artifact |  |  |
| SQLCipher release binary smoke | `lm_node-linux-x86_64-sqlcipher-smoke` artifact from release workflow |  |  |
| Federation validation | `federation-report.json` from `deploy/lm-node-federation/run-all.sh` or workflow artifact |  |  |
| Fuzz smoke | `FUZZ_SMOKE_REPORT=fuzz-smoke-report.json ./scripts/fuzz-smoke.sh` output/report or `./scripts/release-check.sh fuzz-smoke` log |  |  |

## Native node release artifacts

| Artifact | Expected evidence | SHA256 / link | Status |
| --- | --- | --- | --- |
| `lm_node-linux-x86_64.tar.gz` | `RELEASE_INFO.txt`, `.sha256` |  |  |
| `lm_node-linux-x86_64-sqlcipher.tar.gz` | `RELEASE_INFO.txt` with `sqlcipher_enabled=true`, `.sha256`, SQLCipher smoke artifact |  |  |
| `lm_node-macos-x86_64.tar.gz` | `RELEASE_INFO.txt`, `.sha256` |  |  |
| `lm_node-macos-arm64.tar.gz` | `RELEASE_INFO.txt`, `.sha256` |  |  |
| `lm_node-windows-x86_64.zip` | `RELEASE_INFO.txt`, `.sha256` |  |  |
| `SHA256SUMS.txt` | Combined checksum file |  |  |

## Persistence / encryption evidence

| Mode | Required proof | Artifact/link | Status |
| --- | --- | --- | --- |
| SQLCipher `state_db` | `/control/stats` shows `state_db.encryption_mode=sqlcipher` and `state_db.encrypted=true` |  |  |
| SQLCipher metrics | `/control/metrics` includes `lm_node_state_db_encrypted 1` and `lm_node_state_db_encryption_mode{mode="sqlcipher"} 1` |  |  |
| Wrong passphrase fail-closed | `sqlcipher-deploy-smoke.sh` wrong-passphrase check |  |  |
| Encrypted `state_file` if used | stats/metrics show encrypted + permissions hardened |  |  |

## Network / federation evidence

| Scenario | Required proof | Artifact/link | Status |
| --- | --- | --- | --- |
| DHT ContactCard publish/find | federation smoke report/log |  |  |
| Mailbox push/take across nodes | federation smoke report/log |  |  |
| Node outage recovery | chaos smoke report/log |  |  |
| Short Mailbox load | load smoke report/log with `MESSAGE_COUNT` |  |  |
| Public deployment config | sanitized `config.json`, Caddy/reverse-proxy config, node URLs |  |  |

## Security / audit evidence

| Item | Required proof | Artifact/link | Status |
| --- | --- | --- | --- |
| External security audit | audit report and remediation notes |  |  |
| Crypto review | reviewer notes for core/WASM/node control plane |  |  |
| Dependency review | Dependabot / dependency-review status |  |  |
| Signing / notarization | macOS notarization and Windows signing evidence, if production distribution |  |  |
| SECURITY.md review | contact/process verified for release branch |  |  |

## Known limitations accepted for this release

- 

## Final decision

- Release approved by:
- Date/time UTC:
- Notes:
