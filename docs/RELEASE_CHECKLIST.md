# Release Checklist / 发布检查清单

This checklist is the current release-candidate gate for LM Talk. Passing it is **required but not sufficient** for production release: the project still needs production DHT/relay validation, long-running fuzz campaigns, network chaos/load tests, and external security review before claiming production readiness.

## Automated gate

Run the release check from the repository root. The GitHub Actions `CI` workflow runs `./scripts/release-check.sh quick` on pushes and pull requests, so this same gate is enforced remotely:

```bash
./scripts/release-check.sh quick
```

Run dependency vulnerability checks separately (the GitHub Actions `dependency-audit` job does this on pushes and pull requests; `dependency-review` also checks dependency diffs on pull requests):

```bash
./scripts/audit.sh
```

Current audit exceptions in `scripts/audit.sh` are intentionally narrow: `hickory-proto` advisories are ignored because they are pulled into `Cargo.lock` by unused optional `libp2p` DNS/mDNS dependency metadata, while LM Talk only enables TCP/noise/yamux/request-response; `paste` is ignored as a transitive Linux netlink proc-macro warning via `libp2p-tcp`. Revisit these exceptions whenever `libp2p` is upgraded or DNS/mDNS features are enabled.

For a slower local gate that also runs the full Cargo workspace test suite:

```bash
./scripts/release-check.sh full
```

To additionally execute short fuzz smoke runs for every harness:

```bash
./scripts/release-check.sh fuzz-smoke
```

The script currently covers:

- Rust formatting (`cargo fmt --check`).
- `lm_core` unit/e2e/property/test-vector coverage.
- `lm_node` library and binary tests.
- Node e2e flows, including HTTP control plane and Mailbox pressure/failure recovery.
- Fuzz harness compile checks for `core_imports`, `node_dht_rpc`, and `node_control_request`; `fuzz-smoke` mode also starts each target for a short run.
- Web typecheck, production build, and Playwright e2e.


## Tag-based native node artifacts

Native `lm_node` binaries are built by `.github/workflows/release-node.yml` when a Git tag matching `v*` is pushed, or when the workflow is manually dispatched with a tag name. The workflow builds and publishes:

- `lm_node-linux-x86_64.tar.gz`
- `lm_node-linux-x86_64-sqlcipher.tar.gz`
- `lm_node-macos-x86_64.tar.gz`
- `lm_node-macos-arm64.tar.gz`
- `lm_node-windows-x86_64.zip`

Each archive includes the `lm_node` binary, key deployment/security docs, and `RELEASE_INFO.txt` with the source commit, build time, Rust toolchain details, and binary SHA256. The GitHub Release also includes per-artifact `.sha256` files and a combined `SHA256SUMS.txt`.

After the release workflow finishes, verify the published assets and SQLCipher release smoke evidence before sharing the tag:

```bash
./scripts/verify-node-release.sh v0.1.0
```

This downloads the release assets, verifies `SHA256SUMS.txt`, verifies every per-platform `.sha256` file, and checks the archived SQLCipher smoke report proves encrypted `state_db` metrics for the SQLCipher artifact.

To cut a release candidate from the current commit:

```bash
git tag -a v0.1.0 -m "LM Talk node v0.1.0"
git push origin v0.1.0
```

For local artifact smoke checks on Linux after building the target:

```bash
cargo build --locked --release -p lm_node --target x86_64-unknown-linux-gnu
python3 scripts/package-node-release.py \
  --target x86_64-unknown-linux-gnu \
  --package-name lm_node-linux-x86_64 \
  --out-dir dist
```

Current trust caveat: the automated artifacts are not yet macOS-notarized or Windows-code-signed. Treat signing/notarization as required before a production-trust distribution channel.

## Manual release blockers still open

Do not mark the project production-ready until these are explicitly completed and evidenced:

- Production-grade DHT/Kademlia routing/query robustness and public deployment model.
- Relay/TURN replacement strategy that does not become a hard central dependency.
- Long-running fuzz campaigns with saved corpus and crash triage, beyond harness compile checks.
- Real network chaos/load testing: latency, packet loss, reconnects, malformed/hostile peers, sustained Mailbox/DHT load.
- External security audit of core cryptography, Web/WASM bindings, node control plane, and deployment guidance.
- Native node SQLCipher database encryption is implemented behind the `lm_node/sqlcipher` feature and covered by `./scripts/sqlcipher-smoke.sh`; before production-ready release, archive the `lm_node-linux-x86_64-sqlcipher-smoke` artifact from the release workflow or an equivalent deployment run proving the selected release artifact was built with `sqlcipher`, started with `state_db_encryption_mode=sqlcipher`, and reports encrypted state DB metrics. The JSON `state_file` remains only a compatibility/snapshot path.
- Multi-device sync and receipt-state reconciliation beyond backup merge heuristics.

## Evidence to keep for a release candidate

Use `docs/RELEASE_EVIDENCE.md` as the structured evidence index for each release candidate.


Before calling a node build production-ready, also archive evidence for any configured state persistence mode:

- `state_db`: `./scripts/sqlcipher-smoke.sh` output plus the release workflow `lm_node-linux-x86_64-sqlcipher-smoke` artifact (or equivalent deployment evidence) containing `/control/stats` and `/control/metrics` checks for `state_db.encryption_mode=sqlcipher`, `state_db_encrypted=true`, and `lm_node_state_db_encrypted 1` for the exact release artifact/config.
- `state_file`: `/control/stats` and `/control/metrics` showing `state_file.encrypted=true` / `lm_node_state_file_encrypted 1` and `state_file.permissions_hardened=true`; keep secret-file permission checks for the passphrase file.

For every release candidate, archive:

- Output of `./scripts/release-check.sh full`.
- Output/artifact from `./scripts/sqlcipher-smoke.sh`, the manual SQLCipher Smoke workflow, and the release workflow `lm_node-linux-x86_64-sqlcipher-smoke` artifact when SQLCipher state DB encryption is part of the release.
- Fuzz campaign commands, durations, corpus/crash artifacts, and triage notes. Use `./scripts/fuzz-campaign.sh` to generate a JSON campaign report plus per-target logs/corpus/artifact directories.
- Network/load test reports and topology.
- Security-audit report and remediation notes.
- Confirmation that `SECURITY.md` contact/process guidance is current for the release branch.
- Build artifact hashes and deployment configuration used for verification.
- Output of `./scripts/audit.sh` / CI `dependency-audit`.
- For pull requests, CI `dependency-review` status for newly introduced vulnerable dependencies.
- Review status for dependency update PRs generated by Dependabot (`cargo`, Web npm, and GitHub Actions).
