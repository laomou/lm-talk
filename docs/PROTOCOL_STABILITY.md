# Protocol Stability / 协议稳定性

This document defines the compatibility contract for LM Talk protocol objects before a production-ready release. It does not freeze every implementation detail yet; it marks which wire formats are stable enough for interop, which are transitional, and how changes must be introduced.

## Stability levels

- **Stable**: may be relied on by independently deployed clients/nodes. Breaking changes require a new type/version/prefix and migration guidance.
- **Transitional**: usable in current clients but still subject to compatible changes. Breaking changes require explicit release notes and downgrade/interop guidance.
- **Internal/debug**: not a public compatibility surface. Do not depend on it across releases.

## Versioning rules

1. Signed objects must include `type` and `version` in the signed/canonical fields.
2. Exported text prefixes are part of the wire format and must not be reused for incompatible schemas.
3. Incompatible changes require a new `type` or prefix, not silent reinterpretation.
4. Readers should reject unsupported `version` values unless documented as forward-compatible.
5. Size limits are part of the DoS boundary and may become stricter only with a migration note.
6. Expiry fields must be checked by verifiers when present/required by the object type.

## Stable wire objects

These are intended to be stable for release-candidate interop:

| Object | Prefix / type | Stability | Notes |
| --- | --- | --- | --- |
| Identity backup | `lm-identity-backup-v1:` | Stable | Passphrase-protected identity seed package. |
| Contact Card | `lm-contact-card-v1:` | Stable | Signed identity, X25519 key, display name, and device certificates. DHT `ContactCard` records use this payload. |
| Friend request | `lm-friend-request-v1:` | Stable | Signed request, expiry checked. |
| Friend response | `lm-friend-response-v1:` | Stable | Signed response, binds to request/contact. |
| Direct envelope legacy | `lm-direct-envelope-v1:` / `x25519-static-hkdf-xchacha20poly1305-v1` | Transitional | Kept for compatibility; strict E2EE should prefer Ratchet + per-device sealed slots. |
| File package | `lm-file-package-v1:` | Stable | Encrypted file manifest/chunks; filename policy is local. |
| Device cert | `lm-device-cert-v1` | Stable | Signed by identity key; includes device signing public key and device box public key. |
| Device revoke | `lm-device-revoke-v1:` | Stable | Signed by identity key; clients should stop trusting revoked device IDs. |
| Message receipt | `lm-message-receipt-v1:` | Stable | Delivered/read receipts for message IDs and delivery IDs. |
| Mailbox message | `lm-mailbox-message-v1:` | Stable | Node validates sender signature and TTL before storing. |
| Public peer announce | `lm-public-peer-announce-v1:` | Stable | DHT `PublicPeer` records use this payload. |
| PreKey bundle | `lm-prekey-bundle-v1:` | Stable | DHT `PreKey` records use this payload. |
| Signed one-time-prekey record | `lm-signed-one-time-prekey-v1:` | Stable | Used for replenishable one-time keys. |
| MailboxHint DHT value | URL/string value | Transitional | Current record value is an address string; future signed hint object may replace it. |

## Transitional / active-development objects

| Object | Type / prefix | Stability | Required caution |
| --- | --- | --- | --- |
| Double Ratchet state | `lm-ratchet-state-v1:` | Transitional | Local/export state, not a public network message. Changes require local migration. |
| Ratchet envelope | `x3dh-double-ratchet-v1` | Transitional | Current Web path uses it for secure sessions; keep compatibility until protocol freeze. |
| Secure session offer/response | `lm-secure-session-offer-v1`, `lm-secure-session-response-v1` | Transitional | Mailbox-carried setup helper. |
| Group Sender Key distribution | `lm-group-sender-key-v1:` / payload prefix | Transitional | Membership/rotation policy still maturing. |
| Group events | `lm-group-event-v1:` | Transitional | Policy state stable enough for demo, still needs external review. |
| Per-device envelope | `lm-per-device-envelope-v1` | Transitional | Sealed slot crypto is supported; placeholder/fallback is compatibility-only and should be blocked in strict mode. |
| Self-sync package | `lm-self-sync-v1` | Transitional | Signed lightweight same-user state sync; not a message-history sync protocol. |
| Self-sync request | `lm-self-sync-request-v1` | Transitional | Gap repair request/response for recent self-sync packages. |
| Full data backup | `lm-data-backup-v1:` | Transitional | Encrypted backup/restore format for same identity. |
| Node state file | `lm-node-state-file-v1:` | Internal/operator | Native node local persistence format; do not treat as federation wire protocol. |

## DHT record kinds

Current node DHT record kinds and key namespaces are frozen for this release:

| Kind | Key namespace | Value format | Validation |
| --- | --- | --- | --- |
| `PublicPeer` | `public-peer` over `peer_id` | `lm-public-peer-announce-v1:` export text | PublicPeer signature, expiry, and key/peer match. |
| `PreKey` | `prekey` over `user_id` | `lm-prekey-bundle-v1:` export text | PreKey bundle signature, expiry, and key/user match. |
| `MailboxHint` | `mailbox-hint` over `user_id` | Address string | Non-empty, size-limited, accepted URL/multiaddr/mailbox pattern in clients. |
| `ContactCard` | `contact-card` over `user_id` | `lm-contact-card-v1:` export text | ContactCard signature, optional expiry, and key/user match. |

Adding a DHT record kind requires a new namespace, validation rules, max-size review, test vector coverage, release notes, and discovery UI/diagnostics updates.

## Mailbox message kinds

Current mailbox kinds are frozen for this release:

| Kind | Typical payload | Notes |
| --- | --- | --- |
| `SignalOffer` | WebRTC/session offer text | Secure-session helper transport. |
| `SignalAnswer` | WebRTC/session answer text | Secure-session helper transport. |
| `DirectEnvelope` | Direct/Ratchet/per-device envelope payload | Main direct-message delivery path. |
| `GroupFanout` | Per-recipient group envelope/fanout payload | Group delivery path. |
| `DeliveryReceipt` | `lm-message-receipt-v1:` Delivered | Delivery-state reconciliation. |
| `ReadReceipt` | `lm-message-receipt-v1:` Read | Read-state reconciliation. |
| `Other` | Typed payload inspected by prefix/JSON `type` | Compatibility bucket for contact updates, device revokes, secure-session JSON, data backup, self-sync, and future transitional payloads. |

Web maps higher-level local outbox kinds such as `contact-update`, `device-revoke`, `self-sync`, and data backup onto Mailbox `Other`; receivers must inspect payload type/prefix for those higher-level objects. Adding a Mailbox kind requires node validation, Web mapping, backwards-compatible `Other` fallback guidance, and release notes for older nodes.

## Device and ContactCard update policy

1. New devices should create a `lm-device-cert-v1` containing `device_box_public_key`.
2. Contact Cards should preserve known valid device certs by `device_id` when re-exported.
3. Contact Card updates should be distributed through Mailbox `contact-update`, same-user self-sync, and DHT `ContactCard` records.
4. Receivers must preserve local trust state when merging Contact Cards: fingerprint verification, revocations, block state, and read-receipt policy are local decisions.
5. Device revocation wins over stale Contact Card device lists.
6. Strict E2EE mode should require verified contacts and sealed per-device slots for send/receive.

## PreKey rotation policy

1. PreKey bundles may be republished when missing, expired, or low on one-time keys.
2. Signed one-time-prekey records are consumed at most once per node state and consumed state must be snapshotted/synced.
3. DHT `PreKey` records must verify bundle signature and key namespace.
4. Clients should prefer signed one-time-prekey records when available, then fall back to reusable signed prekey behavior.
5. Future incompatible PreKey changes require a new bundle prefix/type.

## Error compatibility

Current user-visible errors are not yet a stable numeric error-code API. Until error codes are frozen:

- Protocol verifiers should return precise typed errors internally.
- Node HTTP endpoints should continue using stable status classes: `400` invalid input, `401` unauthorized, `413` payload too large, `429` rate limited, `5xx` server failures.
- Release candidates must document any changed error text that Web UI depends on.

## Deprecation policy

- Legacy DirectEnvelope and placeholder per-device slots are compatibility paths.
- New strict deployments should enable sealed-slot send/receive and verified-contact policies.
- A future production release may warn by default or disable fallback paths after at least one release cycle with migration notes.

## Production freeze checklist

Before declaring protocol stability for a production release:

- [ ] External audit reviewed all stable and transitional objects above.
- [x] Test vectors exist for stable signed/encrypted objects (see `docs/TEST_VECTOR_COVERAGE.md`).
- [x] DHT record kind list and key derivation namespaces are frozen for the release.
- [x] Mailbox kind mapping and fallback-to-Other behavior are documented.
- [x] ContactCard/DeviceCert merge and revocation policy has interop tests.
- [ ] PreKey rotation/consumption policy has interop tests.
- [ ] Error-code/text dependencies in Web and node tests are documented.
- [ ] Release evidence index links fuzz, federation, SQLCipher, and audit artifacts.
