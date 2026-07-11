# LM Talk Security Model

Goals:

- End-to-end encryption for message/file contents.
- Stable signed identity and contact cards.
- Local-first storage with app-layer encryption for sensitive browser data.
- Best-effort offline delivery without making nodes trusted with plaintext.

Non-goals / limitations:

- No anonymity guarantee.
- Metadata is not fully hidden.
- Web builds have supply-chain risk unless pinned/offline.
- Nodes can deny service, delay, drop, or correlate deliveries.
- Lost identity backup/passphrase cannot be recovered by any platform operator.
