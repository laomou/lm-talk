# Incident Response Runbook / 安全事件响应手册

This runbook defines operational response steps for LM Talk security incidents. It is intended for public-node operators, release owners, and security reviewers.

## Severity levels

- **SEV-1 Critical**: likely compromise of user plaintext, identity private keys, SQLCipher state DB passphrase, release signing keys, or remote code execution on public nodes.
- **SEV-2 High**: node control token leak, device compromise without identity backup leak, DHT poisoning at scale, Mailbox abuse causing service disruption, or bypass of strict E2EE policy.
- **SEV-3 Medium**: localized delivery disruption, stale/invalid DHT records, isolated dependency vulnerability without known exploit, or limited metadata leakage.
- **SEV-4 Low**: diagnostics/doc issues, non-sensitive log exposure, or nuisance abuse.

## First 15 minutes

1. Assign incident commander and scribe.
2. Record UTC start time, reporter, affected version/commit, and affected nodes/clients.
3. Preserve logs and evidence before restarting services where possible.
4. Classify severity and affected component:
   - identity / device / Web client;
   - public node / Mailbox / DHT;
   - release artifact / CI / signing;
   - dependency / supply chain.
5. If active compromise is suspected, contain before triage.

## Evidence to preserve

- Node `/health`, `/control/stats`, `/control/metrics`.
- Node logs for the incident window.
- Sanitized `config.json` and reverse-proxy config.
- Relevant Mailbox delivery IDs and DHT record keys, not decrypted content.
- Release artifact SHA256 and `RELEASE_INFO.txt`.
- Client logs/screenshots with secrets redacted.
- Fuzz crash artifacts or exploit inputs, if applicable.

Never paste identity backups, SQLCipher passphrases, control tokens, or decrypted message content into public issues.

## Incident playbooks

### Identity backup or passphrase leak

Impact: attacker may restore the user's identity and sign objects as that user.

Actions:

1. Treat as SEV-1 for the affected user.
2. Instruct user to stop using the compromised identity for high-trust communication.
3. Create a new identity and verify fingerprints with contacts through an out-of-band channel.
4. Re-establish friendships / ContactCards under the new identity.
5. Revoke old device trust in client UI where possible, but note identity-level compromise cannot be fully repaired by device revoke.
6. Publish warning through trusted channel; do not rely solely on compromised identity.

### Device lost or device private backup leak

Impact: attacker may receive/open sealed slots for that device if they also have required local secrets/backups.

Actions:

1. Generate a `lm-device-revoke-v1` for the lost device.
2. Fan out device revoke to friends.
3. Publish updated ContactCard through Mailbox and DHT.
4. Confirm strict E2EE preflight shows no stale device blockers.
5. Ask contacts to sync and verify revoked device appears in their contact details.

### SQLCipher state DB passphrase leak

Impact: attacker with disk access may decrypt native node state DB.

Actions:

1. Treat as SEV-1 for affected node.
2. Stop node or isolate host.
3. Rotate `state_db_passphrase_file` and create a new SQLCipher state DB from a trusted snapshot if available.
4. Rotate control tokens and sync-peer tokens.
5. Review whether Mailbox deliveries, PreKey state, DHT records, and consumed one-time-prekey state were exposed.
6. Archive `/control/stats` and `/control/metrics` after recovery showing SQLCipher mode.

### Node control token leak

Impact: attacker may access node control APIs and mutate Mailbox/DHT/snapshot state.

Actions:

1. Treat as SEV-2 or SEV-1 if state mutation is confirmed.
2. Add leaked token to `control_previous_tokens` only if a grace window is required; otherwise remove immediately.
3. Generate a new `control_token_file`.
4. Rotate sync peer token references.
5. Review `/control/stats` endpoint counters for unauthorized/bad request spikes.
6. Re-run federation smoke and DHT validation.

### DHT poisoning or malicious peers

Impact: clients may receive invalid records, closer-node abuse, or degraded discovery.

Actions:

1. Capture DHT record keys, bad records, and peer IDs.
2. Confirm validation rejects bad records.
3. Check quarantine metrics and peer health diagnostics.
4. Reset or remove malicious sync peers if they are configured control peers.
5. Re-run DHT maintenance and routing refresh.
6. Publish corrected ContactCard/PreKey/MailboxHint/PublicPeer records.

### Mailbox abuse / quota exhaustion

Impact: users may be unable to receive messages or nodes may degrade.

Actions:

1. Check mailbox quota metrics and reject counters.
2. Increase rate limiting only if current settings are too permissive.
3. Identify abusive sender user IDs from signed Mailbox metadata.
4. Preserve delivery IDs and reject stats.
5. Consider temporary token/CORS restrictions for public endpoint abuse.
6. Re-run load smoke after mitigation.

### Release artifact compromise

Impact: users may run malicious or unverified binaries.

Actions:

1. Treat as SEV-1.
2. Remove affected release artifacts if possible.
3. Publish advisory with bad SHA256 values.
4. Rotate release/signing credentials if compromise suspected.
5. Rebuild artifacts from a clean environment.
6. Re-run release checks, SQLCipher smoke, federation validation, and dependency audit.
7. Publish new release with new checksums and explicit upgrade instructions.

### Dependency vulnerability

Actions:

1. Determine whether vulnerable code is reachable with enabled features.
2. Update dependency or disable feature.
3. Run `./scripts/audit.sh` and release check.
4. If reachable and exploitable, publish advisory and mitigation timeline.
5. Update ignored advisory rationale if an exception remains.

## Communications

Use severity-based communication:

- SEV-1: immediate SECURITY.md contact path, release advisory, direct operator notification.
- SEV-2: security advisory or operator notice within 24 hours.
- SEV-3/4: release notes or issue tracker as appropriate.

Public communications should include:

- affected versions/commits;
- affected deployment modes;
- user/operator action required;
- whether E2EE content confidentiality is believed impacted;
- fixed version or mitigation;
- evidence preservation guidance.

## Recovery validation

Before closing an incident, collect:

- Fix commit(s).
- Test command outputs.
- Relevant smoke report(s): SQLCipher, federation, fuzz if applicable.
- Updated metrics showing healthy state.
- Audit/remediation tracker entry status.
- Release evidence/signoff updates if a release is affected.

## Post-incident review

Within one week:

1. Fill `docs/AUDIT_REMEDIATION_TRACKER.md` entry or issue link.
2. Document root cause and missed detection.
3. Add regression tests or smoke checks.
4. Update runbooks/config defaults if needed.
5. Record accepted residual risk.
