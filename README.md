# LM Talk

A decentralized, end-to-end encrypted instant messaging app. No phone number or email required — your identity is entirely in your hands.

## Features

- **End-to-end encrypted** private and group chats — only you and the recipient can read messages
- **Self-sovereign identity**: create an identity with a passphrase, export/import it freely, no phone or email binding
- **Contact card discovery**: add friends via contact card text or QR code
- **Group chat**: create groups, invite friends, encrypted messaging within groups
- **Offline delivery**: with a sync service configured, messages can be sent and received while the other party is offline — works across LAN and WAN
- **Use anywhere**: web app, just open a browser

## Getting Started

1. Open the web app and **create an identity** with a passphrase (remember to export a backup).
2. Share your **contact card** with friends, or scan theirs to add each other.
3. Start chatting.

To send and receive messages **across devices** or when the **other party is offline**, you need to connect a "sync service": go to "Me → Message Sync" and enter the server address. For self-hosting and cross-device deployment, see [docs/protocol/NETWORK_SPEC.md](docs/protocol/NETWORK_SPEC.md).

## Documentation

For detailed design, specifications, and deployment instructions, see the [documentation index](docs/README.md).

- [Design Overview](docs/overview/DESIGN.md)
- [Security Model](docs/security/SECURITY_MODEL.md)
- [Network & Deployment](docs/protocol/NETWORK_SPEC.md)
- [Protocol Specifications](docs/README.md#核心协议规格)

## License

[Non-Commercial License](LICENSE) © 2026 LM Talk contributors

This project is licensed for non-commercial use only — copying, modification, and distribution are permitted; commercial use requires prior written permission from the authors.
