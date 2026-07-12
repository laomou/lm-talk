# LM Talk 遗留事项 / TODO

版本：v0.1  
日期：2026-07-12  
状态：实现同步草案

本文档记录 `docs/DESIGN.md` 中尚未完全细化的设计决策、协议细节和实现前置任务。

优先级定义：

- **P0**：开始核心编码前必须明确，否则容易返工或产生安全问题。
- **P1**：MVP 阶段需要明确。
- **P2**：正式版或后续增强需要明确。

---

## 当前实现状态快照（2026-07-12）

已完成或基本成型：

- `lm_core`：身份/备份、Contact Card、好友请求/响应、DirectEnvelope、X3DH PreKey、Double Ratchet、群 Sender Key、群权限状态、文件分片加密包、本地安全策略、Outbox、MemoryStore、大小限制、测试向量。
- `lm_wasm`：大部分 core 能力已导出，并有 smoke 测试。
- `lm_node`：HTTP control plane、Public Peer announce、Kademlia ID/distance/closest scaffold、Mailbox push/take/ack、Mailbox TTL/配额/message_id 去重、PreKey publish/get、one-time prekey 消费记录、snapshot sync/import、serve-control 定时 snapshot sync、状态文件保存。
- 测试：`scripts/test.sh all` 当前通过 Rust 测试、core/node e2e、HTTP control flow、WASM smoke、Web build/e2e。

关键边界：

- `lm_node` 仍是控制面 + snapshot sync scaffold，不是真正生产 DHT。
- Mailbox/PreKey 可支撑 demo；Mailbox 已有基础 TTL/配额/message_id 去重，但仍缺正式持久化、认证、自动同步和更完整反滥用。
- Core 协议对象已可测，但仍需属性测试、模糊测试、跨平台测试向量和安全审计。
- 本地持久化仍偏 Web IndexedDB / MemoryStore；Native SQLite/SQLCipher 尚未实现。

---

## 当前未完成功能清单（2026-07-12 更新）

> 当前 `lm_core` / `lm_wasm` / `lm_node` 已具备可测试 MVP scaffold；Web 产品化流程仍是最直接的用户可用性缺口。下面按当前代码状态整理真实缺口。

### P0：让 Web 页面像聊天软件一样可用

1. **正式网络设置区**
   - 在主界面提供简洁设置：`lm_node 控制面 URL`、启用/停用节点、连接状态。
   - 不依赖“Public Peer / Mailbox 协议调试”折叠面板。
   - 设置持久化到 IndexedDB。

2. **自动发布 PreKey**
   - 用户创建/恢复身份后可一键或自动发布自己的 PreKey Bundle 到配置的节点。
   - UI 只显示“已发布/失败/重试”，不暴露大段 JSON。
   - 本地保存 Private PreKey Bundle；不能发给别人。

3. **添加好友后的自动安全建链**
   - 添加联系人后，自动尝试从节点拉取对方 PreKey。
   - 拉取成功后自动创建 X3DH Initial Message / Double Ratchet 初始状态。
   - 没有节点或没有 PreKey 时，继续支持复制粘贴安全会话流程。

4. **离线消息自动走 Mailbox**
   - 单聊发送时：
     - WebRTC DataChannel 已连接：直发。
     - 未连接但配置了节点：自动封装 MailboxMessage 并提交 `/mailbox/push`。
     - 都不可用：保留本地 outbox，并提示可复制密文。
   - 消息状态区分：`queued` / `sent` / `mailbox` / `failed`。

5. **自动收取 Mailbox**
   - 登录后、切回页面、手动点击“收取消息”时调用 `/mailbox/take`。
   - 自动解密 direct-envelope、friend request/response、group fanout、file package。
   - 成功处理后自动 `/mailbox/ack`。
   - UI 只显示“收到 N 条 / 已处理 N 条 / 失败 N 条”。

6. **好友请求走 Mailbox**
   - 对方有 Contact Card / UserID 时，可生成好友请求并通过 Mailbox 投递。
   - 收到好友请求后在正式“好友请求收件箱”显示。
   - 接受/拒绝响应也可通过 Mailbox 返回。

7. **群聊正式收发流程**
   - 群消息 fanout 自动对每个成员发送：WebRTC 在线直发，否则 Mailbox。
   - 群邀请、群事件、Sender Key Distribution 自动进入收件箱并应用。
   - 群成员不是 Friend、被拉黑、缺少密钥时给出明确提示。

8. **文件发送走正式流程**
   - 文件包生成后可自动通过 WebRTC 或 Mailbox 发送。
   - 收到文件包后显示附件卡片，用户点击后再解密/下载。
   - 保持“不自动下载陌生附件”。

9. **正式页面信息架构整理**
   - 左侧：身份摘要、网络状态、联系人、群组。
   - 右侧：聊天头、消息列表、输入框、附件按钮。
   - 把协议 JSON 面板移到“高级/调试”区域，默认不展开。
   - 不加入摄像头扫码功能；二维码只生成和复制原文。

10. **本地数据安全增强**
    - 当前 Web IndexedDB 主要是功能可用路径；需要补应用层加密。
    - 至少加密消息明文、联系人备注、群名、outbox、ratchet session。
    - 只保留必要索引明文。

### P1：可用性与可靠性

1. **Outbox 重试机制**
   - 定时重试 WebRTC / Mailbox 投递。
   - 指数退避、最大重试次数、过期时间、取消发送。

2. **Mailbox 防重复与去重**
   - 本地记录已处理 delivery_id / message_id。
   - 重复拉取不重复显示消息。

3. **节点同步自动化**
   - [x] `serve-control --sync-peer http://host:port --sync-interval-seconds N` 可定时拉取 peer snapshot 并 merge。
   - [ ] 支持多个 sync peer 的持久配置文件和失败退避。
   - [ ] 后续替换为真正 DHT 查询/复制。

4. **联系人更新**
   - 支持 Contact Card 更新 display name、设备列表、PreKey 信息。
   - 禁止静默更换 identity_public_key。

5. **消息 ACK / 送达状态**
   - Mailbox push 成功只代表节点收下，不代表对方已收。
   - 需要送达回执协议；已读回执默认关闭。

6. **多设备基础流程**
   - 新设备导入身份备份后如何同步联系人/消息。
   - 设备证书列表更新和撤销事件自动分发。

7. **PWA / 离线包**
   - Web 版供应链风险提示已存在，但还需要可固定版本离线使用。

### P2：协议与长期增强

1. **真正 DHT / Kademlia 网络**
   - 当前 `lm_node` 是控制面 + snapshot sync scaffold，不是真正 DHT。
   - 需要实现节点发现、closest lookup、记录复制、过期清理。

2. **Relay / TURN 替代能力**
   - 有公网 IP 的节点可选做 bootstrap / DHT / relay / mailbox。
   - Relay 不能成为强中心依赖。

3. **MLS 或更完整群聊协议**
   - 当前群聊是 Sender Key / fanout 实验路径。
   - 大群、成员变更 epoch、历史策略还需完整设计。

4. **生产级身份备份**
   - Web 当前存在 wasm-local 可用性路径；生产要重新做浏览器安全加密备份。
   - 支持改提示词、重新导出、备份完整性校验。

5. **安全审计与测试向量**
   - 补固定协议测试向量。
   - 补属性测试、跨平台一致性测试、浏览器真实流程 E2E。

---

## P0：核心编码前必须明确

### 1. 提示词规范

需要定义提示词的输入、归一化和强度规则。

待决策：

- 是否允许用户自定义提示词。
- 是否默认生成随机词提示词。
- 最低长度或最低强度。
- 是否允许中文、英文、符号混合。
- 是否做弱密码检测。
- 错误提示如何展示。

建议规范：

```text
normalize_passphrase(input):
1. Unicode NFKC 归一化
2. 去除首尾空白
3. 全角空格转半角空格
4. 连续空白合并为一个空格
5. 不静默删除用户输入的普通字符
```

风险：

```text
如果不同平台归一化不一致，用户可能无法解开身份备份包。
```

---

### 2. 身份备份包最终格式

当前 `docs/DESIGN.md` 只有 JSON 示例，需确定最终格式。

待决策：

- 外层使用 JSON、CBOR、postcard 还是自定义二进制。
- 文件扩展名。
- MIME type。
- 是否支持二维码。
- 是否支持复制粘贴文本形式。
- 备份包损坏和提示词错误是否区分提示。

建议：

```text
文件扩展名：.lmid
MIME：application/vnd.lmtalk.identity-backup
文本前缀：lm-identity-backup-v1:
```

推荐编码：

```text
外层可用 JSON 便于调试。
签名或加密输入必须使用 canonical binary encoding。
```

---

### 3. 本机身份缓存策略

需要明确登录时是否必须每次导入身份备份包。

建议：

```text
本机可以保存 encrypted_identity_seed。
用户日常登录只需输入提示词。
新设备恢复必须导入身份备份包。
```

待定义：

- 本机保存格式。
- 本机 encrypted identity 与导出 identity backup 是否相同格式。
- 是否支持重新导出身份备份包。
- 是否支持修改提示词后重新加密身份备份包。

---

### 4. MVP 消息加密握手细节

当前只定义了：

```text
X25519 + HKDF + XChaCha20-Poly1305
```

但需要补充具体协议。

待定义：

- 使用静态 X25519 还是临时 X25519。
- session_id 如何生成。
- shared_secret 如何计算。
- HKDF info/context 如何定义。
- AEAD nonce 如何生成。
- AAD 包含哪些字段。
- message counter 如何维护。
- 如何防重放。
- 如何处理乱序。

建议字段：

```json
{
  "crypto": "x25519-hkdf-xchacha20poly1305-v1",
  "session_id": "base64...",
  "message_counter": 1,
  "nonce": "base64..."
}
```

---

### 5. WebRTC 手动 signaling 包格式

Web MVP 需要手动 offer/answer。

待定义：

- offer 文本格式。
- answer 文本格式。
- ICE candidate 是否包含在 SDP 中。
- 是否使用 trickle ICE。
- 是否签名。
- 是否加密。
- 是否支持二维码。
- 过期时间。

建议文本前缀：

```text
lm-signal-offer-v1:base64url(...)
lm-signal-answer-v1:base64url(...)
```

建议字段：

```json
{
  "type": "lm-signal-offer-v1",
  "version": 1,
  "from_user_id": "lm1_xxx",
  "from_device_id": "dev1_xxx",
  "to_user_id": "lm1_yyy",
  "sdp": "...",
  "created_at": 1783670400,
  "expires_at": 1783671000,
  "signature": "base64..."
}
```

---

### 6. 协议错误码

需要统一错误码，方便 Rust core、WASM 和 UI 协作。

建议错误码：

```text
InvalidSignature
UnsupportedVersion
ExpiredObject
WrongPassphrase
CorruptedBackup
InvalidBackupFormat
BlockedSender
UnknownContact
NotFriend
ReplayDetected
DuplicateMessage
InvalidUserId
InvalidDeviceId
CryptoError
StorageError
NetworkError
PayloadTooLarge
```

待定义：

- 错误码是否稳定对外暴露。
- 是否区分用户可见错误和内部错误。
- 是否避免泄露安全细节。

---

### 7. 协议对象大小限制

需要防止超大对象导致内存或存储攻击。

建议 MVP 限制：

```text
Contact Card：32 KB
Friend Request：64 KB
Friend Request note：1 KB
Direct Message plaintext：64 KB
Group Message plaintext：64 KB
Signaling Offer/Answer：256 KB
Identity Backup：64 KB
```

待定义：

- 超限时错误码。
- Web UI 如何提示。
- 后续文件传输是否走单独协议。

---

### 8. 测试向量格式

协议实现前需要准备固定测试向量。

最低测试向量：

- identity_seed -> UserID
- passphrase + backup -> identity_seed
- Contact Card 签名与验签
- Friend Request 签名与验签
- 消息加密与解密
- 篡改密文失败
- 过期对象拒绝

建议目录：

```text
test-vectors/
  identity_v1.json
  backup_v1.json
  contact_card_v1.json
  friend_request_v1.json
  message_crypto_v1.json
```

---

## P1：MVP 阶段需要明确

### 9. IndexedDB 加密存储策略

Web 端没有 SQLCipher，需要应用层加密。

待定义：

- 哪些字段加密。
- 哪些字段允许明文索引。
- 消息明文是否本地二次加密。
- 联系人昵称是否加密。
- 搜索如何实现。
- IndexedDB schema version 如何迁移。

建议：

```text
敏感内容全部加密。
只保留最小必要明文索引。
```

---

### 10. 数据库迁移机制

需要定义 schema version 和 migration。

待定义：

- meta 表格式。
- migration 文件命名。
- 迁移失败是否回滚。
- 迁移前是否自动备份。
- Web IndexedDB 迁移策略。

建议表：

```sql
CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

---

### 11. Contact Card 更新策略

好友信息可能变化。

允许更新：

- display_name
- avatar
- device list
- signed prekey
- last_seen

不能静默更新：

- user_id
- identity_public_key

待定义：

- 更新包格式。
- 更新包签名。
- 更新频率。
- 设备列表变更如何通知好友。

---

### 12. 信任等级枚举

`contacts.trust_level` 需要明确定义。

建议：

```rust
pub enum TrustLevel {
    Imported,
    LinkImported,
    QrScanned,
    FingerprintVerified,
}
```

待定义：

- 各等级 UI 如何显示。
- 升级信任等级流程。
- 是否允许用户手动标记已验证。

---

### 13. 安全指纹格式

需要定义好友安全码。

建议：

```text
fingerprint = blake3(identity_public_key)[0..16]
显示为：84F2 19AC 77D0 3B91
```

待定义：

- 显示 8 字节还是 16 字节。
- 使用 hex、emoji 还是词组。
- 是否支持扫码比对。

---

### 14. Outbox 重试策略

需要定义离线消息重试逻辑。

待定义：

- 初始重试间隔。
- 指数退避参数。
- 最大重试次数。
- 消息过期时间。
- 用户取消发送。
- 对方被拉黑后如何处理 outbox。

建议：

```text
指数退避：30s -> 2m -> 10m -> 1h -> 6h
默认过期：7 天
```

---

### 15. ACK / 已读 / 正在输入策略

待定义：

- 送达回执是否默认开启。
- 已读回执是否默认关闭。
- 正在输入是否支持。
- 群聊 ACK 如何展示。

建议默认：

```text
送达回执：开启
已读回执：关闭
正在输入：关闭
在线状态：关闭或仅好友可见
```

---

### 16. 群聊顺序和去重

待定义：

- sender_seq 如何维护。
- message_id 如何生成。
- created_at 不可信时如何排序。
- 收到 future epoch 消息如何处理。
- 收到 duplicate message 如何处理。

建议：

```text
每个 sender 在每个 group 内维护 sender_seq。
去重使用 message_id。
展示排序使用 sender_time + received_at 混合。
```

---

### 17. 群聊新人历史策略

待定义：

- 新人默认是否可看历史。
- 是否允许邀请者手动转发最近 N 条。
- 转发历史是否重新加密。
- UI 如何提示。

建议：

```text
新人默认看不到历史。
历史同步必须由老成员手动选择。
```

---

### 18. 本地过滤器 MVP 范围

不要一开始做复杂内容审核。

MVP 建议只做：

- 陌生人附件禁止
- 附件不自动下载
- 外部链接警告
- 可执行文件警告
- 本地拉黑

暂不做：

- 内置敏感词库
- 自动违法内容识别
- 全局举报
- 强制扫描

---

### 20. Web 代码供应链提示

Web 版应明确风险。

待定义：

- 是否支持 PWA 离线模式。
- 是否显示版本 hash。
- 是否支持下载固定离线包。
- 是否计划 Tauri 桌面版。

---

## P2：正式版或后续增强

### 21. Double Ratchet 迁移方案

后续从 MVP 加密升级到 Double Ratchet。

已完成的基础脚手架：

- `RatchetSessionState` 可序列化/反序列化。
- root key、发送链、接收链、DH key、计数器可持久化。
- skipped message keys 有上限并支持乱序接收。
- WASM/Web 调试 API 可创建测试状态对、推进发送/接收链、执行 DH step。

待定义/实现：

- `x3dh-double-ratchet-v1` envelope 格式和核心加解密已完成；Web 正式聊天已在存在 Ratchet Session 时优先使用，还需自动建链。
- session_version 字段。
- 老客户端兼容策略。
- X3DH / signed prekey / one-time prekey 协议对象已完成；还需接入 DHT 发布/领取和 one-time prekey 消耗。
- 会话重建流程。
- skipped message keys 的加密本地存储和清理策略。

建议 envelope 中增加：

```json
{
  "crypto": "x25519-hkdf-xchacha20poly1305-v1"
}
```

未来升级为：

```json
{
  "crypto": "x3dh-double-ratchet-v1"
}
```

---

### 22. X3DH PreKey 体系

已实现：

- `lm-prekey-bundle-v1` 公开包，identity key 签名。
- signed prekey。
- one-time prekeys。
- private prekey bundle 本地保存格式。
- X3DH initial message。
- 发起方/响应方 shared secret 派生测试。

待实现：

- prekey 可发布到 `lm_node` 控制面，并可通过 snapshot 在节点间粗粒度同步；还需真正 DHT 查询/复制。
- DHT 上如何防抢占。
- one-time prekey 消费记录已实现；还需多设备同步和独立 signed one-time-prekey records。
- 复制粘贴版 Ratchet DH public key 交换和 UX 串联已完成；Web 可从 lm_node 拉取 PreKey，还需自动节点发现。
- shared secret 初始化 Double Ratchet 已完成；Web 已持久化 per-contact Ratchet Session，并自动用于发送/接收。

---

### 23. Public Peer Mailbox 细节

Mailbox 是高风险能力，需要防垃圾。

待定义：

- mailbox address 如何生成。
- 写入鉴权。
- 配额。
- TTL。
- 最大消息大小。
- PoW 或 rate limit。
- 是否只接受好友或请求相关消息。
- 拉取后是否删除。
- 多副本策略。

---

### 24. STUN / TURN 策略

待定义：

- 是否默认使用公共 STUN。
- 是否允许用户配置 STUN。
- 是否支持自建 TURN。
- 是否支持 LM Relay 替代 TURN。

建议：

```text
默认可配置 STUN。
TURN 不作为默认官方服务。
用户可自建 TURN/relay。
```

---

### 25. 多设备完整方案

当前只预留 DeviceID。

待定义：

- 新设备如何加入。
- 是否需要旧设备授权。
- 消息是否发给所有设备。
- 多设备消息同步。
- 设备撤销。
- device revoke event。
- device list update event。

注意：

```text
当前模型下，任何拿到身份备份包和提示词的人都可以恢复身份并签发新设备。
```

---

### 26. 完整数据备份

当前只有身份备份包。

还需要设计完整数据备份。

包含：

- 身份
- 联系人
- 好友请求
- 消息
- 群聊
- 拉黑列表
- 本地设置

建议区分：

```text
Identity Backup：身份备份包
Data Backup：完整本地数据备份
```

---

### 27. 文件传输协议

后续需要支持附件。

待定义：

- 文件大小限制。
- 分片大小。
- file_key。
- 文件 hash。
- 断点续传。
- 缩略图是否加密。
- 陌生人附件策略。
- 群文件 fanout 策略。

---

### 28. Sender Key / MLS 群聊升级

MVP 群聊采用逐个加密。

后续可选：

- Sender Key
- MLS

待定义：

- GroupCrypto trait。
- group epoch。
- member add/remove commit。
- 群密钥轮换。
- 老群升级兼容策略。

---

### 29. 可订阅屏蔽列表

本地自治模型可支持用户自愿订阅 blocklist。

待定义：

- blocklist 格式。
- 签名。
- 过期。
- 冲突处理。
- UI 展示。
- 用户如何取消订阅。

---

### 30. 元数据保护增强

当前 E2EE 不保护全部元数据。

后续可选：

- 消息 padding
- 随机延迟
- 固定大小分片
- relay 混淆
- onion routing / Tor / I2P 集成
- DHT 查询混淆

---

## Native Node / 非 Web 后端 TODO

### P0：节点 MVP 稳定化

1. **正式持久化**
   - 为 mailbox deliveries、prekey bundles、consumed one-time prekeys、public peers 增加 SQLite/SQLCipher 或等价存储。
   - 保留 snapshot import/export 作为迁移和调试能力。
   - 增加崩溃恢复测试：push 后崩溃、take 未 ack 后崩溃、ack 后崩溃。

2. **Mailbox 生命周期**
   - [x] TTL 过期清理（push/take/restore/merge 路径会清理过期 delivery）。
   - [x] 基础 per-user / per-node quota（`max_mailbox_messages_per_user` / `max_mailbox_bytes`）。
   - [x] 基础 message_id 去重；delivery_id 去重保留在 snapshot merge 路径。
   - [ ] 持久化 quota/TTL/去重索引，增加崩溃恢复测试。
   - [ ] 更细粒度反滥用：按 sender 限流、全局速率限制、异常 payload 统计。
   - `take` 不删除，只有处理成功后 `ack` 删除的语义已存在，需要持久化和重试测试。

3. **PreKey 生命周期**
   - signed prekey 轮换。
   - one-time prekey 低水位补货。
   - bundle 过期。
   - 后续升级为独立 signed one-time-prekey records，避免 bundle 级签名与消费记录耦合。

4. **控制面安全**
   - 本地开发模式和公网模式分离。
   - CORS 白名单。
   - 管理 API token 或本地 socket 限制。
   - TLS/反向代理部署说明。

### P1：节点自动同步与网络

1. **自动 snapshot sync**
   - [x] CLI 参数配置 peer control URL 列表：`--sync-peer http://a,http://b`。
   - [x] `serve-control` 定时拉取 `/sync/snapshot` 并 merge 到本地节点。
   - [x] 合并 peers/mailbox/prekeys/consumed records 时保持幂等。
   - [ ] 增加配置文件、失败退避、last_sync_at/错误统计。

2. **DHT scaffold 演进**
   - 增加 find_node/find_value/store record 抽象。
   - 为 Public Peer、PreKey record、Mailbox hint 定义 record key。
   - 加记录 TTL、republish、closest-k replication。

3. **节点可观测性**
   - 结构化日志。
   - health/status/stats。
   - mailbox/prekey/peer 数量和过期清理指标。

### P2：生产网络能力

1. **真正 DHT / Kademlia**
   - 节点发现。
   - routing table refresh。
   - record replication。
   - Sybil/垃圾记录基础防护。

2. **Relay / TURN 替代能力**
   - 允许公网节点作为可选 relay/mailbox/bootstrap。
   - Relay 不得成为明文可见或强中心依赖。

3. **节点部署规范**
   - systemd/container 示例。
   - 数据备份/恢复。
   - 升级兼容策略。

---

## 测试计划 TODO

### 单元测试

必须覆盖：

- identity create/restore roundtrip
- wrong passphrase fails
- backup tamper fails
- contact card verify
- invalid contact card rejected
- friend request verify
- expired friend request rejected
- message encrypt/decrypt
- tampered ciphertext fails
- blocked sender dropped
- duplicate message ignored

---

### 属性测试

建议使用 `proptest`。

覆盖：

- 任意 payload encode/decode 一致。
- 任意字节篡改导致验签或解密失败。
- 过期时间边界。
- 大小限制边界。

---

### WASM 测试

需要验证：

- Web RNG 可用。
- Argon2id 参数在浏览器可接受。
- IndexedDB 存取可用。
- WASM 与 native 生成一致 UserID。
- WASM 与 native 测试向量一致。

---

### 跨平台测试向量

必须确保以下内容跨平台一致：

- passphrase normalization
- backup decrypt
- UserID generation
- Contact Card signature
- Friend Request signature
- message encryption

---

## 法律与产品边界 TODO

需要在用户协议或 README 中明确：

```text
1. 用户对自己发送和接收的内容负责。
2. 软件不托管公开内容。
3. 软件不承诺全局内容过滤。
4. 软件不提供全局封禁。
5. 软件不保证匿名。
6. 软件不保证消息一定送达。
7. 端到端加密保护内容，但不隐藏所有元数据。
8. 无服务器意味着身份丢失后无法由平台找回。
```

---

## 建议拆分的后续规范文件

当前 `docs/DESIGN.md` 是总设计文档。后续建议继续拆分：

```text
docs/
  DESIGN.md
  IDENTITY_SPEC.md
  BACKUP_SPEC.md
  CONTACT_SPEC.md
  FRIEND_SPEC.md
  MESSAGE_SPEC.md
  GROUP_SPEC.md
  NETWORK_SPEC.md
  PUBLIC_PEER_SPEC.md
  STORAGE_SPEC.md
  SECURITY_MODEL.md
  MVP_PLAN.md
```

---

## 当前最高优先级清单

建议下一步优先完成：

```text
1. Web 正式网络设置区：lm_node URL、启停、连接状态、IndexedDB 持久化。
2. PreKey 自动发布/拉取/补货：隐藏 JSON 调试细节，保留 private bundle 本地加密保存。
3. 添加好友后自动 X3DH + Double Ratchet 建链，失败时回退复制粘贴流程。
4. Mailbox 自动发送、收取、解密、ack、去重和失败重试。
5. 本地数据应用层加密：消息明文、联系人备注、群名、outbox、ratchet session。
6. Native node 正式持久化：SQLite/SQLCipher 或等价数据库，含过期清理和崩溃恢复测试。
7. 节点自动同步增强：配置文件、失败退避、同步状态指标；后续替换为 DHT replication。
8. Outbox 调度器：指数退避、取消发送、过期、delivery status。
9. 协议稳定化：错误码、对象大小限制、Contact Card 更新策略、PreKey 轮换策略。
10. 安全测试增强：proptest/fuzz、跨平台测试向量、ratchet replay/window/skipped-key 不变量。
```
