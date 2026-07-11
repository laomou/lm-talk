# LM Talk Contact Spec v1

Contact Cards are signed identity advertisements:

```text
lm-contact-card-v1:<base64url-json>
```

Signed fields include UserID, display name, identity public key, X25519 public key, device certificates, creation time, and optional expiry. Clients may update display name and device certificates only when `user_id` and `identity_public_key` match the existing contact. Silent identity key replacement is forbidden.

Trust levels: Imported, LinkImported, QrScanned, FingerprintVerified.
Fingerprint display uses a short BLAKE3-derived hexadecimal code.

See `test-vectors/contact_card_v1.json`.
