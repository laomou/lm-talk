# Crypto Review Notes / 密码学审计说明

This document is a concise guide for reviewers of LM Talk's cryptographic design and implementation. It summarizes key lifecycle, trust boundaries, and known limitations. It should be read together with `docs/SECURITY_AUDIT_SCOPE.md`, `docs/PROTOCOL_STABILITY.md`, and `docs/TEST_VECTOR_COVERAGE.md`.

## Identity and root keys

- Each user identity owns an Ed25519 signing key and X25519 public key derived from the identity seed.
- Identity backups are passphrase protected and exported with `lm-identity-backup-v1:`.
- Web local storage derives an application storage key from the restored identity and encrypts sensitive IndexedDB records.
- Re-encrypting the identity backup rotates the local IndexedDB encryption key and rewrites local records.
- User IDs are derived from identity public keys; verifiers reject mismatched user/public-key pairs.

## Contact trust boundary

- Contact Cards are signed identity objects (`lm-contact-card-v1:`) containing identity public keys, X25519 public keys, and device certificates.
- Fingerprint verification is a local trust decision. Remote ContactCard updates must not overwrite local fingerprint verification state.
- Block state, read-receipt policy, revocation state, and local safety policy are local state and must survive ContactCard merges.
- DHT ContactCard records are public discovery records; they are integrity protected by ContactCard signatures but not private.

## Device lifecycle

- Device certificates (`lm-device-cert-v1`) are signed by the identity key.
- Device certs include:
  - device signing public key;
  - `device_box_public_key` for per-device sealed slot encryption;
  - device ID derived from the device signing public key.
- Device revocations (`lm-device-revoke-v1:`) are signed by the identity key.
- Revocation wins over stale ContactCard device lists.
- Own device certificates propagate through same-user self-sync, ContactCard Mailbox updates, and DHT ContactCard publishing.

## X3DH and PreKey handling

- PreKey bundles (`lm-prekey-bundle-v1:`) are signed by the identity key.
- Signed one-time-prekey records (`lm-signed-one-time-prekey-v1:`) are independently signed and tied to bundle/user/signed-prekey IDs.
- Nodes persist consumed one-time-prekey IDs and merge consumed state through snapshots.
- Clients prefer signed one-time-prekey records when available and fall back to reusable signed prekey behavior.
- PreKey DHT records are validated against DHT key namespace and bundle signature.

## Direct messages and Ratchet

- Legacy DirectEnvelope uses static X25519 + HKDF + XChaCha20-Poly1305. This is a compatibility path, not the preferred strict deployment path.
- Ratchet state and message keys are covered by deterministic vectors. Ratchet state export is sensitive local state.
- Strict deployments should use Ratchet sessions delivered inside per-device sealed slots.
- Replay and skipped-message-key bounds are enforced in Ratchet state transitions.

## Per-device sealed slots

- Per-device envelope v1 (`lm-per-device-envelope-v1`) is a signed outer package listing target device slots.
- Sealed slots use X25519 ephemeral DH + HKDF + XChaCha20-Poly1305 with slot AAD binding:
  - conversation id;
  - sender user id;
  - target device id;
  - created_at.
- The outer envelope signature covers target devices, fallback ciphertext, and metadata.
- Placeholder/fallback slots exist only for compatibility; strict mode rejects non-sealed inbound/outbound paths.
- Missing `device_box_public_key` should be treated as a downgrade risk.

## Group encryption

- Group Sender Key state uses HKDF chain advancement and XChaCha20-Poly1305 sender envelopes.
- Group membership events define when sender-key rotation is required.
- Group event policy currently remains transitional and requires external review before protocol freeze.
- New members do not automatically receive historical messages; history transfer must be explicit and re-encrypted.

## File encryption

- File chunks are encrypted independently with static X25519 + HKDF + XChaCha20-Poly1305.
- File manifests include hash, name, MIME type, size, chunk size, and chunk count.
- Filename risk handling is local UI policy; cryptographic verification covers encrypted chunk integrity and file hash verification.

## Self-sync

- Self-sync packages (`lm-self-sync-v1`) and requests (`lm-self-sync-request-v1`) are signed by the identity key.
- Self-sync is a lightweight same-user state protocol, not a full message-history sync channel.
- It carries contact/device/trust state, DHT history, receipt states, outbox summaries, own device certs, and gap-repair metadata.
- Sequence and previous-sync IDs support dedupe and gap detection.
- Recent packages are cached temporarily for gap repair.

## Local persistence

- Web IndexedDB sensitive records are application-layer encrypted.
- Native `state_db` supports:
  - plain mode;
  - external encrypted volume mode;
  - SQLCipher mode behind the `lm_node/sqlcipher` feature.
- SQLCipher mode uses `state_db_passphrase_file`, `PRAGMA key`, and `PRAGMA cipher_version` validation.
- Wrong SQLCipher passphrases must fail closed and must not silently create a fresh node state.
- JSON `state_file` supports XChaCha20-Poly1305 application-layer encryption for compatibility/snapshot workflows.

## Node trust boundary

- Mailbox nodes are not trusted with plaintext. They store signed encrypted payloads and metadata.
- DHT nodes are not trusted. Records are validated by kind-specific signatures/key namespaces.
- Control-plane tokens protect node APIs except `/health`; operators must configure CORS and TLS correctly.
- Node metrics should not leak message plaintext, private keys, or decrypted payloads.

## Metadata limitations

LM Talk does not currently hide all metadata. The following may be visible to nodes or observers depending on transport/deployment:

- user IDs and DHT keys;
- mailbox delivery timing and size;
- record kinds and publication timing;
- node peer IDs and addresses;
- approximate traffic volume;
- ContactCard public fields and device counts.

This system is content-confidential and integrity-protected, not anonymous or traffic-analysis resistant.

## Reviewer focus questions

1. Are all signed canonical fields complete and stable for each object?
2. Can any remote update overwrite local trust decisions?
3. Can a malicious node trigger downgrade from sealed slots to fallback paths under strict policy?
4. Can device revocation be bypassed by stale ContactCards or self-sync packages?
5. Can one-time-prekeys be reused across node snapshot merges?
6. Do Web/WASM boundaries expose raw secrets longer than necessary?
7. Do SQLCipher and state_file fail closed on missing/wrong passphrases?
8. Do DHT validation failures quarantine/penalize malicious peers without blocking honest recovery?
9. Are size limits and parsing errors sufficient for malformed payloads?
10. Are release artifacts and deployment templates aligned with the intended security mode?

## Strict E2EE control-message exception

Strict E2EE send policy is fail-closed for user content: direct messages, files, group messages, group events, group invitations, secure-session payloads, and normal outbox retries must satisfy verified-contact and sealed-slot requirements before sending.

Two outbound control-message families intentionally bypass the strict send-content gate because they are required to repair or revoke trust state:

- `lm-contact-card-v1:` / `contact-update` fanout, used to distribute fresh ContactCards, device certificates, and `device_box_public_key` values needed to reach sealed-slot readiness.
- `lm-device-revoke-v1`, used to stop peers from trusting a lost or retired device.

These exceptions do not carry message/file plaintext. They remain signed protocol objects and should be reviewed as trust-state repair paths, not as content-delivery downgrade paths. Inbound content still enforces verified-contact and sealed-slot policy, and strict mode blocks risky group creation, group invites, group messages, group events, file send/decrypt, secure sessions, and mailbox receipts.
