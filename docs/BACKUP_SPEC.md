# LM Talk Backup Spec v1

Identity backups are text-prefixed JSON objects encoded as base64url without padding:

```text
lm-identity-backup-v1:<base64url-json>
```

The JSON object contains type/version/user_id, Argon2id parameters, and an XChaCha20-Poly1305 encrypted identity seed. Backup AEAD AAD is fixed by the core crypto module. Wrong passphrase and corrupted backup are distinct internal errors, but UI should avoid leaking excessive detail.

File extension recommendation: `.lmid`.
MIME recommendation: `application/vnd.lmtalk.identity-backup`.

See `test-vectors/backup_v1.json`.
