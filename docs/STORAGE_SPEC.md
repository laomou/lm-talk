# LM Talk Storage Spec v1

Web storage uses IndexedDB tables. Sensitive fields are encrypted at application layer with AES-GCM using a PBKDF2-derived local key based on normalized passphrase and UserID.

Encrypted-at-rest fields include message text, contact display name/card, group names, outbox payloads, and ratchet session state. Minimal routing/index fields remain plaintext to keep the UI usable.

Schema migrations must preserve old localStorage and single-object IndexedDB imports.
