# LM Talk Public Peer / Mailbox Spec v1

Public peer announcements advertise peer id, addresses, capabilities, quotas, and expiry. Mailbox messages are signed by sender identity and include recipient UserID, kind, ciphertext payload, creation time, and expiry.

Mailbox kinds include SignalOffer, SignalAnswer, DirectEnvelope, GroupFanout, and Other. Nodes verify signatures and store deliveries until clients take and ack them.

Anti-abuse requirements for production: quotas, TTL, max message size, rate limits or proof-of-work, and duplicate suppression.
