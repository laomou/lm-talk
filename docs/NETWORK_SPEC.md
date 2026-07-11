# LM Talk Network Spec v1

Networking is optional and best-effort.

- WebRTC DataChannel is preferred for online direct delivery.
- Mailbox is used for offline delivery through configured `lm_node` control plane.
- Outbox retries use exponential backoff: 30s, 2m, 10m, 1h, 6h; default expiry is 7 days.
- Snapshot sync can pull `/sync/snapshot` from a peer node and import it locally.

Current node networking is a control-plane scaffold with closest-peer and snapshot support, not a production DHT.
