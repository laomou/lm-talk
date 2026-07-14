# LM Talk Backup Spec v1

Identity backups are text-prefixed JSON objects encoded as base64url without padding:

```text
lm-identity-backup-v1:<base64url-json>
```

The JSON object contains type/version/user_id, Argon2id parameters, and an XChaCha20-Poly1305 encrypted identity seed. Backup AEAD AAD is fixed by the core crypto module. Wrong passphrase and corrupted backup are distinct internal errors, but UI should avoid leaking excessive detail.

Browser WASM currently also accepts a local compatibility prefix:

```text
lm-identity-backup-v1:wasm-local:<base64url-json>
```

This path keeps Web identity creation usable while Argon2id backup encryption is not required in every browser runtime. It still uses Web RNG for the identity seed, normalized passphrase input for local key derivation, AEAD for the encrypted seed, and a UserID consistency check on restore. Native/core backups remain the standard Argon2id format above.

File extension recommendation: `.lmid`.
MIME recommendation: `application/vnd.lmtalk.identity-backup`.

See `test-vectors/backup_v1.json`.
