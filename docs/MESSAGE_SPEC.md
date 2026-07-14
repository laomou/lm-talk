# LM Talk 消息规格 v1

Direct message envelope 类型：`lm-direct-envelope-v1`。

支持的 crypto ID：

- `x25519-static-hkdf-xchacha20poly1305-v1`：用于 MVP 兼容路径。
- `x3dh-double-ratchet-v1`：用于 ratchet 会话。

Envelope AAD 包含 type、version、crypto id、message id、sender、recipient、created_at、nonce，以及存在时的 ratchet header。明文在 AEAD 加密前使用 canonical binary encoding。

Mailbox 投递 ACK 使用本地 JSON 载荷类型 `lm-delivery-ack-v1`，并将本地消息状态更新为 delivered。已读回执默认关闭。

测试向量见 `test-vectors/message_crypto_v1.json`。
