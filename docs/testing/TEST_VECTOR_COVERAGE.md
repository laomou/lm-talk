# Test Vector Coverage / 测试向量覆盖

This file tracks which protocol objects have stable cross-platform test vectors and which still need fixtures before LM Talk can claim protocol stability.

## Existing vectors

| Fixture | Covered object(s) | Test file | Status |
| --- | --- | --- | --- |
| `test-vectors/identity_v1.json` | deterministic identity seed, user id, Ed25519 public key, X25519 public key, storage key, passphrase normalization | `crates/lm_core/tests/test_vectors.rs`, `crates/lm_wasm/src/lib.rs` | Covered |
| `test-vectors/backup_v1.json` | identity backup export text, seed restore, wrong-passphrase rejection, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/contact_card_v1.json` | Contact Card export text, signature verification, display name, user id, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/friend_request_v1.json` | Friend request export text, signature verification, from/to user ids, note, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/device_v1.json` | deterministic device seed, device cert JSON, device box public key, device revoke text, signature verification, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/receipt_mailbox_v1.json` | delivered/read message receipts and signed mailbox message, signature verification, parsed fields, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/prekey_v1.json` | PreKey bundle, signed one-time-prekey records, key IDs, signature verification, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/ratchet_v1.json` | deterministic shared secret, ratchet initial keys, first sending/receiving message key, exported states, replay rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/contact_card_dht_v1.json` | DHT ContactCard key derivation, record JSON, signed ContactCard value, store validation, wrong-key rejection | `crates/lm_node/src/lib.rs` | Covered |
| `test-vectors/public_peer_v1.json` | PublicPeer announce export text, DHT key derivation, record JSON, signature verification, wrong-key rejection | `crates/lm_node/src/lib.rs` | Covered |
| `test-vectors/per_device_envelope_v1.json` | per-device envelope v1 outer shape, sealed slot metadata, AAD target binding, legacy fallback slot marker | `apps/web/tests/ui-smoke.spec.ts` | Covered |
| `test-vectors/self_sync_v1.json` | self-sync package/request shape, sequence/gap fields, receipt state summary, outbox summary, own device fields | `apps/web/tests/ui-smoke.spec.ts` | Covered |
| `test-vectors/file_package_v1.json` | file manifest, encrypted file chunk, decrypt verification, hash verification, ciphertext tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/group_v1.json` | group invite, rename event, add-member event, signature verification, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/group_sender_key_v1.json` | group sender key distribution, sender envelope, decrypt verification, replay/tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/message_crypto_v1.json` | legacy DirectEnvelope encryption/decryption, conversation id, plaintext, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |

## Missing stable vectors before protocol freeze

No High-priority or Medium-priority protocol vector gaps are currently listed. New stable wire objects must be added to the Existing vectors table or explicitly documented here before protocol freeze.

## Acceptance criteria for new vectors

A new vector should include:

1. A deterministic seed or fixed input where possible.
2. The exported text / JSON wire object.
3. Public identifiers and key material required for verification.
4. Expected plaintext or parsed fields.
5. A tamper case in tests, not necessarily in the fixture file.
6. Coverage from native Rust tests, and WASM tests when the object crosses Web/WASM boundaries.

## Release checklist linkage

Before marking protocol stability complete:

- Every object listed as **Stable** in `docs/PROTOCOL_STABILITY.md` should either have a vector above or a documented reason why it cannot be deterministic.
- Missing High-priority vectors should be treated as release blockers for production protocol freeze; currently none are listed.
- `docs/RELEASE_EVIDENCE.md` should link the test run proving these vectors pass for the release commit.
