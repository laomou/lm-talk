# LM Talk Message Spec v1

Direct message envelope type: `lm-direct-envelope-v1`.

Supported crypto IDs:

- `x25519-static-hkdf-xchacha20poly1305-v1` for MVP compatibility.
- `x3dh-double-ratchet-v1` for ratchet sessions.

Envelope AAD includes type, version, crypto id, message id, sender, recipient, created_at, nonce, and ratchet header when present. Plaintext is canonical binary encoded before AEAD encryption.

Mailbox delivery ACK uses local JSON payload type `lm-delivery-ack-v1` and updates local message state to delivered. Read receipts remain off by default.

See `test-vectors/message_crypto_v1.json`.
