# PublicPeer / Mailbox 规格 v1

PublicPeer 公告描述一个可发现节点：

```text
lm-public-peer-announce-v1:<base64url-json>
```

公告包含 peer id、地址、能力、配额、创建时间、过期时间和签名。

## Mailbox 消息

Mailbox 消息由发送方身份签名，包含：

- message id；
- from / to User ID；
- kind；
- 密文载荷；
- 创建时间和过期时间；
- 签名。

节点只验证签名、大小、TTL、配额和去重，不解密载荷。

## Mailbox kind

常见 kind：

- `SignalOffer`
- `SignalAnswer`
- `DirectEnvelope`
- `GroupFanout`
- `DeliveryReceipt`
- `ReadReceipt`
- `Other`

## 反滥用

节点应配置 TTL、最大消息大小、每用户配额、全局/发送者限流和重复消息抑制。

测试向量：`test-vectors/public_peer_v1.json`。
