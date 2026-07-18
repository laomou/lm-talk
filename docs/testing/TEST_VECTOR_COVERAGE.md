# 测试向量覆盖

本文件跟踪哪些协议对象已有稳定的跨平台测试向量，哪些仍需在 LM Talk 声称协议稳定前补齐测试夹具。

## 现有向量

| 夹具 | 覆盖对象 | 测试文件 | 状态 |
| --- | --- | --- | --- |
| `test-vectors/identity_v1.json` | 确定性身份种子、用户 ID、Ed25519 公钥、X25519 公钥、存储密钥、口令规范化 | `crates/lm_core/tests/test_vectors.rs`, `crates/lm_wasm/src/lib.rs` | Covered |
| `test-vectors/backup_v1.json` | 身份备份导出文本、种子恢复、错误口令拒绝、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/contact_card_v1.json` | Contact Card 导出文本、签名验证、显示名、用户 ID、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/friend_request_v1.json` | 好友请求导出文本、签名验证、发起/接收用户 ID、备注、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/device_v1.json` | 确定性设备种子、设备证书 JSON、设备箱公钥、设备撤销文本、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/receipt_mailbox_v1.json` | 送达/已读消息回执和签名 Mailbox 消息、签名验证、解析字段、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/prekey_v1.json` | PreKey bundle、签名一次性 PreKey 记录、密钥 ID、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/ratchet_v1.json` | 确定性共享秘钥、Ratchet 初始密钥、首次发送/接收消息密钥、导出状态、重放拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/contact_card_dht_v1.json` | DHT ContactCard 键派生、记录 JSON、签名 ContactCard 值、存储验证、错误键拒绝 | `crates/lm_node/src/lib.rs` | Covered |
| `test-vectors/public_peer_v1.json` | PublicPeer 公告导出文本、DHT 键派生、记录 JSON、签名验证、错误键拒绝 | `crates/lm_node/src/lib.rs` | Covered |
| `test-vectors/per_device_envelope_v1.json` | 每设备信封 v1 外形、封闭槽元数据、AAD 目标绑定、旧版回退槽标记 | `apps/web/tests/ui-smoke.spec.ts` | Covered |
| `test-vectors/self_sync_v1.json` | 自同步包/请求形状、顺序/缺口字段、回执状态摘要、发件箱摘要、自有设备字段 | `apps/web/tests/ui-smoke.spec.ts` | Covered |
| `test-vectors/file_package_v1.json` | 文件清单、加密文件分片、解密验证、哈希验证、密文篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/group_v1.json` | 群组邀请、重命名事件、添加成员事件、签名验证、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/group_sender_key_v1.json` | 群组发送密钥分发、发送者信封、解密验证、重放/篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |
| `test-vectors/message_crypto_v1.json` | 旧版 DirectEnvelope 加密/解密、会话 ID、明文、篡改拒绝 | `crates/lm_core/tests/test_vectors.rs` | Covered |

## 在协议冻结前缺失的稳定向量

当前未列出任何高优先级或中优先级协议向量缺口。新的稳定线对象必须添加到上表，或在协议冻结前明确记录为不可确定的原因。

## 新向量的接受标准

新向量应包括：

1. 在可能情况下使用确定性种子或固定输入。
2. 导出文本 / JSON 线对象。
3. 验证所需的公有标识符和密钥材料。
4. 预期明文或解析字段。
5. 一个篡改案例，测试文件中不一定要包含在夹具内。
6. 由原生 Rust 测试覆盖；当对象跨越 Web/WASM 边界时也应有 WASM 测试。

## 发布检查清单链接

在将协议稳定性标记完成前：

- `docs/PROTOCOL_STABILITY.md` 中标记为 **Stable** 的每个对象要么在上表中已有向量，要么有文档说明为什么不能确定。
- 丢失的高优先级向量在生产协议冻结前应视为阻塞项；当前未列出。
- `docs/RELEASE_EVIDENCE.md` 应链接证明这些向量在发布提交上通过的测试运行。
