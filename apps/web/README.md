# LM Talk Web MVP

Vue + TypeScript + Rust WASM demo.

## Run

From repository root, development uses direct npm commands:

```bash
export PATH="$PWD/.tools/node/bin:$PATH"
cd apps/web
npm run dev
```

Production build:

```bash
./scripts/build.sh web
```

The project-local Node.js binary is installed under:

```text
.tools/node/bin
```

## Current features

- Create/restore identity
- Export identity backup text
- Create device cert
- Export/inspect contact card
- Friend request/response text exchange
- Encrypt/decrypt MVP direct message envelope
- Manual WebRTC offer/answer and DataChannel
- LocalStorage draft save/load


## Limitations

- Direct message crypto is currently an MVP scheme, not Signal Double Ratchet.
- Manual WebRTC signaling is used; there is no DHT discovery yet.
- Offline delivery is not implemented.
- Group chat uses pairwise fanout envelopes.
- Camera scanning is intentionally not used; QR codes are generation-only.
- IndexedDB persistence is currently whole-state KV storage, not normalized tables.

## Product and legal boundaries

1. Users are responsible for the content they send, receive, store, export, and share.
2. LM Talk does not host public content; optional nodes only relay/sync protocol objects such as mailbox deliveries, peer snapshots, and prekeys.
3. LM Talk does not provide global moderation, global reporting, or global account bans. Local blocking and local safety filters are device-local controls.
4. End-to-end encryption protects message content, but it does not hide all metadata such as timing, peer/node usage, mailbox delivery existence, or contact relationships implied by traffic.
5. Serverless/local-first operation means identity recovery is impossible without the identity backup package and passphrase.
6. Message delivery is best-effort. WebRTC, mailbox, outbox retries, and acknowledgements improve reliability but do not guarantee delivery or reading.
7. Device certificates and revoke events are signed by the identity key; users should distribute updated Contact Cards and revoke events to friends after adding or losing devices.
8. The web/PWA build has normal web supply-chain risk. Prefer pinned/offline builds for sensitive use.
