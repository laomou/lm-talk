# 测试向量覆盖

本文件记录稳定协议对象的测试向量覆盖情况。测试向量位于仓库根目录 `test-vectors/`。

## 已覆盖向量

| 夹具 | 覆盖对象 | 主要测试位置 | 状态 |
| --- | --- | --- | --- |
| `identity_v1.json` | 确定性身份种子、User ID、Ed25519/X25519 公钥、存储密钥、提示词归一化 | `crates/lm_core/tests/test_vectors.rs`, `crates/lm_wasm/src/lib.rs` | 已覆盖 |
| `backup_v1.json` | 身份备份导出、恢复、错误口令拒绝、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `contact_card_v1.json` | ContactCard 导出、签名验证、显示名、User ID、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `friend_request_v1.json` | 好友请求、签名验证、发送/接收用户、备注、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `device_v1.json` | 设备证书、device box 公钥、设备撤销、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `receipt_mailbox_v1.json` | Delivered/Read receipt、签名 Mailbox message、字段解析、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `prekey_v1.json` | PreKey bundle、signed one-time prekey、key id、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `ratchet_v1.json` | Ratchet 共享密钥、初始密钥、收发密钥、导出状态、重放拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `contact_card_dht_v1.json` | ContactCard DHT key、record JSON、签名 value、错误 key 拒绝 | `crates/lm_node/src/lib.rs` | 已覆盖 |
| `public_peer_v1.json` | PublicPeer announce、DHT key、record JSON、签名验证、错误 key 拒绝 | `crates/lm_node/src/lib.rs` | 已覆盖 |
| `per_device_envelope_v1.json` | per-device envelope v1 外形、sealed slot 元数据、AAD 绑定、兼容 fallback 标记 | `apps/web/tests/ui-smoke.spec.ts` | 已覆盖 |
| `self_sync_v1.json` | self-sync package/request 形状、sequence、gap、回执/outbox/设备字段 | `apps/web/tests/ui-smoke.spec.ts` | 已覆盖 |
| `file_package_v1.json` | 文件 manifest、加密分片、解密、hash、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `group_v1.json` | 群邀请、重命名、添加成员、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `group_sender_key_v1.json` | Sender Key 分发、sender envelope、解密、重放/篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |
| `message_crypto_v1.json` | 旧 DirectEnvelope 加解密、会话 ID、明文、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | 已覆盖 |

## 新增向量标准

新增稳定 wire object 时，应同时提供：

1. 固定输入或确定性种子。
2. 导出文本或 JSON wire object。
3. 公开标识符和验签所需公钥。
4. 预期解析字段或明文。
5. 至少一个篡改拒绝用例。
6. Rust 测试覆盖；跨 Web/WASM 边界的对象也应有 Web/WASM 覆盖。

## 注意事项

部分向量包含过期时间。测试应允许对象在当前时间已过期时被 `verify()` 正确拒绝，同时仍检查字段解析和格式稳定性。
