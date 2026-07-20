# 消息规格 v1

直接消息使用 DirectEnvelope 或 Ratchet envelope。旧 DirectEnvelope 是兼容路径；新身份默认 strict E2EE 时应优先使用 Ratchet + per-device sealed slot。

## DirectEnvelope

类型：

```text
lm-direct-envelope-v1
```

支持的 crypto id：

| crypto id | 说明 |
| --- | --- |
| `x25519-static-hkdf-xchacha20poly1305-v1` | 旧兼容路径。 |
| `x3dh-double-ratchet-v1` | Ratchet 会话消息。 |

Envelope AAD 绑定 type、version、crypto id、message id、发送方、接收方、created_at、nonce 和可选 ratchet header。

## 回执

当前稳定回执对象：

```text
lm-message-receipt-v1:
```

支持：

- Delivered
- Read

回执必须绑定目标消息 ID、会话 ID 和可选 Mailbox delivery ID。strict 接收策略会拒绝未核验或已撤销设备联系人的回执。

## strict E2EE

strict 模式下：

- 发送要求联系人指纹已核验；
- 发送要求目标活跃设备支持 sealed slot；
- 接收要求联系人已核验；
- 接收要求 sealed slot 入站。

测试向量：`test-vectors/message_crypto_v1.json`。
