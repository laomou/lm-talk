# Public Deployment Report Template / 公网部署报告模板

Use this template after running a real public LM Talk federation drill. It should be linked from `docs/RELEASE_EVIDENCE.md` and `docs/RELEASE_SIGNOFF.md` before any production-ready claim.

## Deployment identity

- Report ID:
- Date/time UTC:
- Operator:
- Release/tag:
- Commit SHA:
- Web origin(s):
- Node artifact(s):
- SQLCipher artifact used: yes/no

## Topology

| Node | Domain | Region/provider | Peer ID | Role | Version/commit | SQLCipher mode | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A |  |  |  | bootstrap + DHT + mailbox |  |  |  |
| B |  |  |  | DHT + mailbox |  |  |  |
| C |  |  |  | DHT + mailbox |  |  |  |

## Configuration evidence

Attach sanitized copies or links:

- [ ] Node A `config.json`
- [ ] Node B `config.json`
- [ ] Node C `config.json`
- [ ] Reverse proxy / TLS config for each node
- [ ] Firewall/security-group summary
- [ ] CORS allowlist
- [ ] Token rotation plan
- [ ] Backup/restore plan

## Health and metrics

| Node | `/health` archived | `/control/stats` archived | `/control/metrics` archived | Status |
| --- | --- | --- | --- | --- |
| A |  |  |  |  |
| B |  |  |  |  |
| C |  |  |  |  |

Required checks:

- [ ] Every node serves HTTPS.
- [ ] Only `/health` is unauthenticated.
- [ ] Authenticated endpoints reject missing/wrong bearer token.
- [ ] Metrics do not contain message plaintext, identity backups, tokens, or decrypted payloads.

## SQLCipher / persistence evidence

For each node using SQLCipher:

| Node | `state_db.encryption_mode` | `state_db.encrypted` | `lm_node_state_db_encrypted` | Wrong passphrase test | Status |
| --- | --- | --- | --- | --- | --- |
| A |  |  |  |  |  |
| B |  |  |  |  |  |
| C |  |  |  |  |  |

Attach:

- [ ] `sqlcipher-deploy-smoke.log` or equivalent
- [ ] `/control/stats` evidence
- [ ] `/control/metrics` evidence
- [ ] Wrong-passphrase fail-closed output

## DHT validation

| Record kind | Published from | Found from | Evidence | Status |
| --- | --- | --- | --- | --- |
| ContactCard |  |  |  |  |
| PreKey |  |  |  |  |
| MailboxHint |  |  |  |  |
| PublicPeer |  |  |  |  |

Required checks:

- [ ] Valid records are found from at least one other node.
- [ ] Invalid/mismatched records are rejected.
- [ ] DHT maintenance/replication output archived.
- [ ] No repeated quarantine of healthy peers.

## Mailbox validation

| Scenario | Evidence | Status |
| --- | --- | --- |
| Push signed message to node A |  |  |
| Take from node A |  |  |
| Snapshot/import to node B |  |  |
| Take recovered message from node B |  |  |
| Ack delivery and verify tombstone/status |  |  |
| Quota/rate-limit behavior under short load |  |  |

## Federation / chaos / load

| Drill | Command/report | Duration/count | Result | Notes |
| --- | --- | --- | --- | --- |
| Basic smoke |  |  |  |  |
| Node outage recovery |  |  |  |  |
| Short load |  |  |  |  |
| Longer load/partition test |  |  |  |  |

Attach `federation-report.json`, logs, and any load-test summaries.

## Client/Web validation

| Flow | Evidence | Status |
| --- | --- | --- |
| Web client connects to all nodes |  |  |
| ContactCard DHT publish/find |  |  |
| PreKey DHT publish/find |  |  |
| Mailbox send/receive |  |  |
| Strict E2EE mode send/receive |  |  |
| Device cert update/ACK convergence |  |  |
| Self-sync receipt/outbox summaries |  |  |

## Incidents and anomalies

List any unexpected errors, warnings, quarantined peers, rate-limit spikes, failed mailbox deliveries, or DHT lookup failures.

| Time UTC | Node/client | Symptom | Root cause | Resolution | Follow-up |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |

## Go / no-go conclusion

- Decision: `GO` / `NO-GO`
- Blocking issues:
- Accepted risks:
- Required follow-up tickets:
- Operator sign-off:
- Security reviewer sign-off:
- Date/time UTC:

A deployment report is **NO-GO** as production evidence if any node lacks HTTPS, SQLCipher evidence is missing for a SQLCipher deployment, cross-node Mailbox recovery fails, DHT ContactCard/PreKey discovery fails, or logs show repeated panics during the validation window.
