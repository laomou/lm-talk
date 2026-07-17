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
| `test-vectors/message_crypto_v1.json` | legacy DirectEnvelope encryption/decryption, conversation id, plaintext, tamper rejection | `crates/lm_core/tests/test_vectors.rs` | Covered |

## Missing stable vectors before protocol freeze

These objects have unit/e2e coverage but do not yet have committed cross-platform fixtures in `test-vectors/`:

| Object | Needed vector evidence | Priority |
| --- | --- | --- |
| PublicPeer announce | `lm-public-peer-v1:` text, peer id, addresses, capabilities, DHT key namespace | Medium |
| File package | encrypted file package, manifest, chunk decrypt, tamper rejection | Medium |
| Group invite/event | invite text, group event text, policy transition, signature verification | Medium |
| Group sender key distribution | distribution text, sender key state import/export, sender envelope decrypt | Medium |
| Ratchet session/envelope | deterministic shared secret/session state, encrypt/decrypt, skipped key behavior | High |
| Per-device envelope v1 | signed outer envelope, sealed slot metadata, slot open with device backup, fallback rejection in strict mode | High |
| Self-sync package/request | signed `lm-self-sync-v1`, request package, sequence/gap fields, signature verification | Medium |
| ContactCard DHT record | DHT `ContactCard` record key/value fixture and verification | Medium |

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
- Missing High-priority vectors should be treated as release blockers for production protocol freeze.
- `docs/RELEASE_EVIDENCE.md` should link the test run proving these vectors pass for the release commit.
