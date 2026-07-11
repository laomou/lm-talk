# LM Talk Identity Spec v1

Identity is derived from a 32-byte `identity_seed`.

- Ed25519 identity signing key: HKDF(identity_seed, `lm-talk.identity.ed25519.v1`).
- X25519 static key: HKDF(identity_seed, `lm-talk.identity.x25519.v1`).
- UserID: `lm1_` plus a stable base32/blake3 digest of the Ed25519 public key.
- Local storage key: HKDF(identity_seed, storage context) and used only for local encrypted state.

Passphrases are normalized before backup KDF:

1. Unicode NFKC.
2. Trim leading/trailing whitespace.
3. Convert full-width spaces to ASCII spaces.
4. Collapse consecutive whitespace to one ASCII space.
5. Do not silently remove normal user characters.

See `test-vectors/identity_v1.json`.
