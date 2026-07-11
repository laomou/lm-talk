# LM Talk Friend Request Spec v1

Friend request text:

```text
lm-friend-request-v1:<base64url-json>
lm-friend-response-v1:<base64url-json>
```

A request contains the sender Contact Card, target UserID, optional note, creation time, expiry, and identity signature. A response signs request id, sender/receiver IDs, accept/reject, and creation time.

Requests and responses may be copied manually, shown as QR text, sent over WebRTC, or delivered through Mailbox as `Other` payloads.

See `test-vectors/friend_request_v1.json`.
