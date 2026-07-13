# LM Talk 去中心化聊天软件设计文档

版本：v0.1  
日期：2026-07-12  
状态：实现同步草案

---

## 1. 项目定位

LM Talk 是一个无中心服务器、P2P 直连、端到端加密、本地加密存储的聊天系统。

核心目标：

- 不依赖手机号、邮箱、中心账号服务器。
- 用户身份由本地随机密钥决定。
- 恢复身份需要 **身份备份包 + 提示词**。
- 好友关系必须双方确认。
- 消息只在用户设备之间加密传输。
- 聊天记录只存本地。
- 没有社区管理员，没有全局审核，没有全局封禁。
- 安全策略由用户本地客户端执行。

一句话定义：

> LM Talk 是一个以身份备份包和提示词为身份恢复机制，以好友双向确认为社交边界，以本地自治为安全模型的无服务器 P2P 端到端加密聊天系统。

---

## 2. 核心原则

### 2.1 无中心服务器

系统不依赖中心服务器提供以下能力：

- 账号注册
- 身份认证
- 好友关系存储
- 消息存储
- 消息转发
- 内容审核
- 全局封禁

后续可以允许用户自建或选择公共节点，但公共节点不应成为可信中心。

---

### 2.2 端到端加密

所有聊天消息在发送端加密，在接收端解密。

中间节点只能看到密文，不能读取消息内容。

---

### 2.3 本地优先

以下数据均保存在本地：

- 身份密钥
- 设备密钥
- 好友列表
- 好友请求
- 消息记录
- 群聊状态
- 本地拉黑列表
- 本地安全策略
- 待发送队列

本地数据库需要加密。

---

### 2.4 身份自持

用户身份不由服务器签发，而由本地随机生成的 `identity_seed` 决定。

```text
identity_seed -> identity keypair -> identity_public_key -> UserID
```

---

### 2.5 好友双向确认

导入对方身份不等于好友成立。

好友关系必须经过：

```text
Friend Request -> Friend Response
```

双方确认后才进入 `Friend` 状态。

---

### 2.6 本地自治安全

系统没有社区管理者，没有全局审核者。

安全策略由用户本地客户端执行：

- 好友白名单
- 陌生人请求箱
- 本地过滤
- 本地拉黑
- 附件风险提示
- 可选订阅屏蔽列表

---

## 3. 总体架构

```text
┌──────────────────────────────┐
│ Web UI / Mobile UI / Desktop │
└───────────────▲──────────────┘
                │
┌───────────────┴──────────────┐
│ lm_wasm / FFI Binding        │
└───────────────▲──────────────┘
                │
┌───────────────┴──────────────┐
│ lm_core                      │
│ identity / contact / message │
│ crypto / group / policy      │
└───────────────▲──────────────┘
                │
┌───────────────┴──────────────┐
│ storage adapter              │
│ IndexedDB / SQLite/SQLCipher │
└───────────────▲──────────────┘
                │
┌───────────────┴──────────────┐
│ network adapter              │
│ WebRTC / DHT / Public Peer   │
└──────────────────────────────┘
```

### 3.1 当前实现状态快照（2026-07-12）

当前仓库已经从纯设计推进到可测试 MVP scaffold。非 Web 部分状态如下：

| 模块 | 当前状态 | 完整性判断 |
|---|---|---|
| `lm_core` | 已实现身份、备份、Contact Card、好友请求/响应、DirectEnvelope、X3DH PreKey、Double Ratchet 状态与 envelope、群 Sender Key、群权限状态、文件分片加密包、本地安全策略、Outbox、MemoryStore、大小限制、测试向量 | 核心协议层已具备 MVP 主干，约 75-85% MVP 完整；仍需生产级审计、持久化接口、属性/模糊测试、多设备完整流程 |
| `lm_wasm` | 已暴露大部分 core API，覆盖身份、联系人、好友、消息、PreKey/X3DH、Ratchet、群、文件、Public Peer、Mailbox、Signaling | 绑定层覆盖较全，约 70-80% MVP 完整；仍需随 core API 稳定后整理命名、错误码和兼容策略 |
| `lm_node` | 已实现控制面 HTTP scaffold、Public Peer announce、Kademlia ID/XOR distance/closest peers、DHT record key/value scaffold 与控制面 store/find/closest、DHT RPC 消息/本地处理 scaffold 与 `POST /dht/rpc` 入口、closest-k replication plan 与 routing refresh target plan 及控制面 plan 端点、control-peer StoreRecord replication runner、control-peer FindNode routing refresh runner、可信返回节点合并及其 stats/metrics、Mailbox push/take/ack、Mailbox TTL/配额/message_id 去重、PreKey publish/get、独立 signed one-time prekey records 发布/同步/消费、PreKey 过期清理/轮换重置/低水位提示、snapshot sync/import、serve-control 定时 snapshot sync、控制面 token/CORS 基础安全、控制面 per-client IP 基础限流、`/control/stats` JSON 运行指标、`/control/metrics` OpenMetrics 文本导出、DHT replication/routing refresh runner 指标、过期清理维护统计、状态文件原子保存、mailbox state-file 崩溃恢复测试 | 可支撑节点辅助 PreKey + Mailbox + 粗粒度同步 demo，约 71-75% MVP 完整；不是生产 DHT/relay 节点 |
| CLI / 运维 | 已有 `announce`、`inspect-public`、`distance`、`run`、`serve-control`、`--config-file`、`--control-token`、`--control-token-file`、`--cors-allow-origin`、`--rate-limit-*`、DHT runner 配置项，以及 `docs/NODE_CONFIG.md` / `docs/examples/lm-node.config.example.json` | 调试和基础部署可用；仍缺 TLS 文档、日志、数据库、后台任务 |
| 测试 | `scripts/test.sh all` 覆盖 Rust fmt/test、core e2e、node e2e、HTTP control flow、WASM smoke、Web build/e2e | 基础回归较好；仍需 proptest/fuzz、跨实现向量、真实网络故障/压力测试 |

重要边界：

- 当前 `lm_node` 的 Kademlia 部分已有 ID、距离、bucket、closest 查询和 record key/value scaffold；**尚未实现真正 DHT 网络查询、节点发现、跨节点记录复制和 routing table refresh**。
- Mailbox 当前是控制面队列语义：节点可保存密文并等待接收方 ack；已有基础 TTL、配额、message_id 去重、控制面 per-client IP 限流、按 sender/全局限流和 SQLite state_db 持久化，尚未包含端到端投递回执、更强反滥用和元数据保护。
- PreKey 当前支持 bundle 发布/拉取、独立 signed one-time-prekey records 和 one-time key 精确消费记录；后续还需真正 DHT 查询/复制、多设备补货协调和审计。
- Core 中的加密协议对象和状态机已经可测试，但仍不能等同于经过第三方审计的生产安全协议。

---

## 4. 技术选型

### 4.1 核心语言

使用 Rust。

原因：

- 适合实现长期可复用核心库。
- 内存安全。
- 适合加密协议和状态机。
- 可编译到 WebAssembly。
- 后续可通过 FFI 支持移动端。

---

### 4.2 Web 端

建议：

- React / Vue / Svelte + TypeScript + Vite
- Rust WASM
- Browser WebRTC
- IndexedDB

Web 端职责：

- UI
- WebRTC 调用
- 本地浏览器存储
- 调用 WASM 完成身份、签名、加密、解密

---

### 4.3 Native 后续版本

后续 Native 节点可使用：

- Rust
- SQLite / SQLCipher
- rust-libp2p
- Kademlia DHT
- mDNS
- WebRTC / QUIC
- 可选 relay / mailbox

---

### 4.4 Rust 依赖建议

核心 crate 可考虑：

```text
argon2
hkdf
blake3
ed25519-dalek
x25519-dalek
chacha20poly1305
serde
postcard
wasm-bindgen
zeroize
secrecy
uuid
base64
bs58
```

---

## 5. 仓库结构建议

```text
lm-talk/
  Cargo.toml

  crates/
    lm_core/
      src/
        lib.rs
        error.rs
        identity/
        contact/
        friend/
        message/
        group/
        crypto/
        policy/
        protocol/

    lm_wasm/
      src/
        lib.rs

    lm_storage/
      src/
        lib.rs
        traits.rs
        memory.rs
        indexeddb.rs
        sqlite.rs

    lm_network/
      src/
        lib.rs
        traits.rs
        webrtc.rs
        dht.rs
        relay.rs
        mailbox.rs

    lm_node/
      src/
        lib.rs

  apps/
    web/
      package.json
      src/
        main.ts
        App.tsx

    cli/
      src/
        main.rs
```

---

## 6. 身份系统设计

### 6.1 最终身份模型

身份不由提示词单独决定。

采用：

```text
identity_seed = 本地随机生成
passphrase = 用户提示词
identity_backup = 用 passphrase 加密 identity_seed
UserID = hash(identity_public_key)
```

恢复身份必须同时拥有：

```text
身份备份包 + 提示词
```

提示词只是解锁身份备份包的密码，不直接决定 UserID。

---

### 6.2 创建身份流程

```text
1. 用户输入提示词
2. 客户端随机生成 identity_seed
3. 从 identity_seed 派生身份密钥对
4. 生成 identity_public_key
5. 根据 identity_public_key 生成 UserID
6. 用提示词通过 Argon2id 派生 backup_key
7. 用 backup_key 加密 identity_seed
8. 导出身份备份包
9. 创建本地加密数据库
```

---

### 6.3 恢复身份流程

```text
1. 用户导入身份备份包
2. 用户输入提示词
3. 客户端用 Argon2id 派生 backup_key
4. 解密 identity_seed
5. 重新派生身份密钥对
6. 重新计算 UserID
7. 校验 UserID 是否与备份包一致
8. 恢复成功
```

---

### 6.4 身份备份包

技术名：

```text
lm-identity-backup-v1
```

示例：

```json
{
  "type": "lm-identity-backup-v1",
  "version": 1,
  "user_id": "lm1_x7k2p9af3m6qz0n4b8r1",
  "kdf": {
    "name": "argon2id",
    "salt": "base64...",
    "memory_kib": 65536,
    "iterations": 3,
    "parallelism": 1
  },
  "cipher": {
    "name": "xchacha20poly1305",
    "nonce": "base64...",
    "ciphertext": "base64..."
  },
  "created_at": 1783670400
}
```

`ciphertext` 解密后：

```json
{
  "identity_seed": "base64...",
  "created_at": 1783670400
}
```

---

### 6.5 UserID 生成

```text
identity_seed
  ↓ HKDF("lm-talk.identity.ed25519.v1")
identity_private_key
  ↓
identity_public_key
  ↓
UserID = "lm1_" + base32(blake3(identity_public_key))[0:40]
```

---

### 6.6 提示词角色

提示词不是身份本身。

提示词只用于：

```text
加密/解密 identity_seed
```

因此：

```text
同一句提示词不会生成同一个 UserID，除非 identity_seed 也相同。
```

---

### 6.7 安全提醒

必须向用户说明：

```text
身份备份包 + 提示词 = 完整身份控制权。
丢失备份包且本机丢失 = 无法恢复身份。
泄露备份包但未泄露提示词 = 仍需破解提示词。
泄露备份包和提示词 = 身份被接管。
```

---

## 7. 设备系统设计

### 7.1 UserID 与 DeviceID

系统区分：

```text
UserID   = 用户身份
DeviceID = 设备身份
```

一个 UserID 可以有多个 DeviceID。

---

### 7.2 设备密钥

每台设备首次登录时本地随机生成：

```text
device_seed = random(32 bytes)
device_keypair = Ed25519(device_seed)
DeviceID = "dev1_" + base32(blake3(device_public_key))[0:40]
```

---

### 7.3 设备证书

设备由用户身份私钥签名。

```json
{
  "type": "lm-device-cert-v1",
  "version": 1,
  "user_id": "lm1_xxx",
  "device_id": "dev1_xxx",
  "device_public_key": "base64...",
  "device_name": "Alice Phone",
  "created_at": 1783670400,
  "signature_by_identity_key": "base64..."
}
```

验证逻辑：

```text
1. 根据 user_id 找到 identity_public_key。
2. 验证 signature_by_identity_key。
3. 确认 device_id == hash(device_public_key)。
```

---

### 7.4 MVP 设备策略

MVP 可以先支持单设备，但协议字段必须预留：

- device_id
- device_public_key
- device_cert

---

## 8. 好友系统设计

### 8.1 好友不是服务器关系

好友关系只保存在本地。

加好友的本质是：

```text
导入对方 Contact Card -> 校验身份 -> 发送好友请求 -> 等待对方确认
```

---

### 8.2 Contact Card

技术名：

```text
lm-contact-card-v1
```

示例：

```json
{
  "type": "lm-contact-card-v1",
  "version": 1,
  "user_id": "lm1_alice_xxx",
  "display_name": "Alice",
  "identity_public_key": "base64...",
  "x25519_public_key": "base64...",
  "device_certs": [
    {
      "device_id": "dev1_phone_xxx",
      "device_public_key": "base64...",
      "signature_by_identity_key": "base64..."
    }
  ],
  "created_at": 1783670400,
  "expires_at": 1786262400,
  "signature": "base64..."
}
```

---

### 8.3 Contact Card 校验

导入时必须校验：

```text
1. UserID == hash(identity_public_key)
2. card signature 有效
3. device cert signature 有效
4. 字段版本支持
5. created_at / expires_at 合理
```

---

### 8.4 好友状态

```rust
pub enum ContactState {
    LocalOnly,
    RequestSent,
    RequestReceived,
    Friend,
    Rejected,
    Blocked,
}
```

含义：

| 状态 | 含义 |
|---|---|
| LocalOnly | 我导入了对方身份，但未发送请求 |
| RequestSent | 我已发送好友请求 |
| RequestReceived | 对方请求添加我 |
| Friend | 双方已确认 |
| Rejected | 已拒绝 |
| Blocked | 本地拉黑 |

---

### 8.5 好友请求

```json
{
  "type": "lm-friend-request-v1",
  "version": 1,
  "request_id": "uuid",
  "from_user_id": "lm1_bob_xxx",
  "to_user_id": "lm1_alice_xxx",
  "from_contact_card": {},
  "note": "我是 Bob",
  "created_at": 1783670400,
  "expires_at": 1784275200,
  "signature": "base64..."
}
```

好友请求必须签名。

---

### 8.6 好友响应

```json
{
  "type": "lm-friend-response-v1",
  "version": 1,
  "request_id": "uuid",
  "from_user_id": "lm1_alice_xxx",
  "to_user_id": "lm1_bob_xxx",
  "accepted": true,
  "created_at": 1783670500,
  "signature": "base64..."
}
```

---

### 8.7 完整好友流程

```text
Bob 获取 Alice Contact Card
Bob 校验 Alice 身份
Bob 本地保存 Alice = LocalOnly
Bob 发送 Friend Request
Bob 本地状态 = RequestSent

Alice 收到请求
Alice 校验 Bob Contact Card
Alice 显示请求箱
Alice 点击接受
Alice 本地保存 Bob = Friend
Alice 返回 Friend Response

Bob 收到响应
Bob 状态变为 Friend
```

---

## 9. 消息系统设计

### 9.1 消息加密路线

MVP 阶段：

```text
X25519 + HKDF + XChaCha20-Poly1305
```

当前新增 X3DH / PreKey 脚手架：

```text
lm-prekey-bundle-v1:
- identity public key / identity X25519 public key
- signed_prekey_id / signed_prekey_public_key
- identity signature

lm-signed-one-time-prekey-v1:
- user_id / identity_public_key
- signed_prekey_id / key_id / public_key
- created_at / expires_at
- identity signature

lm-x3dh-initial-message-v1:
- initiator identity X25519 public key
- initiator ephemeral public key
- selected signed_prekey_id
- optional one_time_prekey_id
```

Rust/WASM/Web 调试区已能生成公开 PreKey Bundle、独立 signed one-time-prekey records、保存 private prekey bundle、验签、发起方派生 shared secret、响应方派生同一个 shared secret。Shared secret 现在可以初始化 `RatchetSessionState`，并已新增实验性 `x3dh-double-ratchet-v1` envelope 加解密路径。Web IndexedDB 已增加 per-contact ratchet session 保存；正式聊天发送/接收会在存在会话时自动走 ratchet，不存在时回退 MVP。Web 已新增复制粘贴版“安全会话建立”UX：Offer 携带 PreKey Bundle 和 Ratchet DH public key，Response 携带 X3DH initial message 和响应方 Ratchet DH public key。Native node 控制面已支持 `/prekey/publish`、`/prekey/get`、`/prekey/status`、`/sync/snapshot`、`/sync/import`，Web 可发布/拉取 PreKey Bundle 与 signed one-time-prekey records，并可在两个节点之间粗粒度同步 peers/mailbox/prekeys 快照。下一步是真正的 DHT 路由复制、开放传输层查询和多设备补货协调。

当前新增 Double Ratchet 状态脚手架：

```text
lm-ratchet-state-v1:
- session_id
- root_key
- local/remote DH public key
- local DH private key（只允许本地加密保存）
- send_chain_key / recv_chain_key
- send_count / recv_count / previous_send_count
- skipped_message_keys
```

实验性消息 Envelope：

```text
crypto = x3dh-double-ratchet-v1
ratchet_header = { session_id, dh_public_key, previous_send_count, message_number }
message_key = ratchet send/recv chain 派生
payload = PlainMessage，经 XChaCha20-Poly1305 加密
```


它已经可在 Rust/WASM/Web 调试区创建、导入导出、推进发送链/接收链、保存乱序 skipped keys，并执行 DH ratchet step。正式消息路径已在存在 ratchet session 时优先使用，建链仍需更多自动化；还需要会话重建和重放窗口策略。

正式阶段：

```text
X3DH + Signal Double Ratchet
```

群聊正式阶段可考虑：

```text
Sender Key 或 MLS
```

---

### 9.2 消息 Envelope

外层：

```json
{
  "type": "lm-direct-envelope-v1",
  "version": 1,
  "message_id": "uuid",
  "from_user_id": "lm1_alice_xxx",
  "from_device_id": "dev1_phone_xxx",
  "to_user_id": "lm1_bob_xxx",
  "to_device_id": "dev1_laptop_xxx",
  "created_at": 1783670400,
  "ciphertext": "base64..."
}
```

---

### 9.3 明文 Payload

解密后：

```json
{
  "type": "lm-message-v1",
  "version": 1,
  "message_id": "uuid",
  "conversation_id": "conv_xxx",
  "sender_user_id": "lm1_alice_xxx",
  "body": {
    "kind": "text",
    "text": "你好"
  },
  "created_at": 1783670400
}
```

---

### 9.4 消息状态

```rust
pub enum MessageStatus {
    Draft,
    Queued,
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
    Expired,
}
```

---

### 9.5 ACK

```json
{
  "type": "lm-ack-v1",
  "version": 1,
  "message_id": "uuid",
  "from_user_id": "lm1_bob_xxx",
  "received_at": 1783670450,
  "signature": "base64..."
}
```

默认建议：

- 送达回执：可开启。
- 已读回执：默认关闭。
- 正在输入：默认关闭。
- 在线状态：默认隐藏或仅好友可见。

---

## 10. 群聊设计

### 10.1 MVP 群聊模型

MVP 使用：

```text
小群 + 好友邀请 + 对方确认 + 逐个加密发送
```

即 Pairwise Fanout。

Alice 给群发消息时：

```text
Encrypt(Alice -> Bob, group_message)
Encrypt(Alice -> Carol, group_message)
Encrypt(Alice -> Dave, group_message)
```

---

### 10.2 群聊限制

MVP 规则：

```text
1. 只能邀请已确认好友
2. 被邀请人必须手动接受
3. 群人数默认上限 20
4. 不支持公开群搜索
5. 不支持群链接任意加入
6. 不支持管理员治理
7. 不做自动 gossip 转发
```

---

### 10.3 GroupID

```text
group_id = "grp1_" + base32(random 32 bytes)
```

---

### 10.4 群成员状态

```rust
pub enum GroupMemberStatus {
    Invited,
    Joined,
    Left,
    Removed,
}
```

---

### 10.5 群邀请

```json
{
  "type": "lm-group-invite-v1",
  "version": 1,
  "invite_id": "uuid",
  "group_id": "grp1_xxx",
  "group_name": "测试群",
  "inviter_user_id": "lm1_alice_xxx",
  "members": [
    "lm1_alice_xxx",
    "lm1_bob_xxx",
    "lm1_carol_xxx"
  ],
  "created_at": 1783670400,
  "signature": "base64..."
}
```

---

### 10.6 群消息

解密后的群消息：

```json
{
  "type": "lm-group-message-v1",
  "version": 1,
  "group_id": "grp1_xxx",
  "epoch": 1,
  "message_id": "uuid",
  "sender_user_id": "lm1_alice_xxx",
  "sender_seq": 12,
  "body": {
    "kind": "text",
    "text": "大家好"
  },
  "created_at": 1783670600,
  "signature": "base64..."
}
```

---

### 10.7 群状态一致性

无服务器、无管理员情况下：

```text
群状态不是强一致的。
每个用户拥有自己的本地群视图。
```

用户可以本地屏蔽某个成员，但不会全局踢出该成员。

---

## 11. 本地自治安全模型

### 11.1 不做全局管理

系统不提供：

- 全局管理员
- 社区审核
- 服务器内容扫描
- 全局封禁
- 全局删除
- 公开举报中心

---

### 11.2 默认安全策略

默认采用好友白名单模式。

规则：

```text
1. 只有 Friend 状态联系人可直接聊天
2. 陌生人只能发好友请求
3. 陌生人不能发附件
4. 陌生人不能邀请入群
5. 群聊只能邀请好友
6. 附件默认不自动下载
7. 外部链接默认警告
8. 可执行文件默认警告
```

---

### 11.3 本地安全策略结构

```rust
pub struct LocalSafetyPolicy {
    pub stranger_messages: StrangerMessagePolicy,
    pub allow_stranger_attachments: bool,
    pub allow_stranger_group_invites: bool,
    pub auto_download_media: bool,
    pub warn_external_links: bool,
    pub warn_executable_files: bool,
    pub enable_text_filter: bool,
    pub text_filter_level: FilterLevel,
}
```

默认值：

```rust
LocalSafetyPolicy {
    stranger_messages: StrangerMessagePolicy::FriendRequestOnly,
    allow_stranger_attachments: false,
    allow_stranger_group_invites: false,
    auto_download_media: false,
    warn_external_links: true,
    warn_executable_files: true,
    enable_text_filter: true,
    text_filter_level: FilterLevel::Standard,
}
```

---

### 11.4 本地拉黑

```rust
pub struct BlockEntry {
    pub user_id: UserId,
    pub reason: Option<String>,
    pub created_at: u64,
}
```

拉黑后：

```text
1. 丢弃该用户私聊消息
2. 隐藏该用户群消息
3. 拒绝该用户好友请求
4. 拒绝该用户群邀请
5. 不下载该用户附件
6. 不向该用户转发群消息
```

---

### 12.1 Web MVP

Web 第一版不强行做完整 DHT。

采用：

```text
手动 signaling + Browser WebRTC DataChannel
```

流程：

```text
Alice 生成 offer
Bob 粘贴 offer
Bob 生成 answer
Alice 粘贴 answer
WebRTC DataChannel 建立
双方发送加密消息
```

---

### 12.2 Native 后续版本

后续 Rust native node 可加入：

- rust-libp2p
- Kademlia DHT
- DHT peer announce
- DHT mailbox
- WebRTC signaling
- mDNS 局域网发现
- 用户自建 relay

---

### 12.3 WebRTC 限制

WebRTC 需要信令通道。

无服务器条件下，可选：

- 手动复制
- 二维码
- DHT mailbox
- 好友中继
- 自建中继

---

### 12.4 NAT 限制

无 TURN 中继时，连接可能失败。

产品必须明确：

```text
LM Talk 默认尽力直连。
在严格 NAT、防火墙、移动网络环境下，可能无法连接。
```

---

## 13. Public Peer 设计

### 13.1 定义

有公网 IP 或可稳定被连接的节点可以充当 Public Peer。

它不是传统服务器，而是可选的 P2P 增强节点。

Public Peer 可以提供以下能力：

```text
bootstrap
DHT routing
signaling
relay
mailbox
```

---

### 13.2 Public Peer 能力等级

#### Level 0：普通节点

```text
无公网 IP。
只能主动连接别人。
不能稳定被连接。
```

#### Level 1：Bootstrap / DHT Node

```text
有公网 IP。
长期在线。
参与 DHT。
帮助新节点加入网络。
```

#### Level 2：Signaling Node

```text
提供 WebRTC 信令邮箱。
暂存 offer / answer / ICE。
TTL 很短。
```

#### Level 3：Relay Node

```text
帮 NAT 失败的节点转发密文流量。
不解密，不存储。
```

#### Level 4：Mailbox Node

```text
短期暂存离线密文消息。
需要配额、TTL、防垃圾策略。
```

---

### 13.3 Public Peer 公告格式

```json
{
  "type": "lm-public-peer-announce-v1",
  "version": 1,
  "peer_id": "peer1_xxx",
  "device_id": "dev1_xxx",
  "user_id": "lm1_xxx",
  "addresses": [
    "/ip4/1.2.3.4/tcp/4001",
    "/ip4/1.2.3.4/udp/4001/quic"
  ],
  "capabilities": [
    "bootstrap",
    "dht",
    "signaling",
    "relay"
  ],
  "limits": {
    "max_mailbox_bytes": 10485760,
    "max_message_ttl_seconds": 86400,
    "max_relay_bandwidth_kbps": 1024
  },
  "created_at": 1783670400,
  "expires_at": 1783674000,
  "signature": "base64..."
}
```

---

### 13.4 Public Peer 信任模型

Public Peer 默认不可信。

必须假设它可能：

- 丢消息
- 延迟消息
- 返回假数据
- 记录元数据
- 拒绝服务
- 审查某些用户
- 注入垃圾记录

客户端必须：

- 验签
- 端到端加密
- 使用短 TTL
- 使用多节点冗余
- 失败切换
- 不把私钥交给 Public Peer
- 不信任 Public Peer 返回的身份数据

---

### 13.5 网络连接策略

```text
1. 优先直连
2. 直连失败尝试 NAT hole punching
3. hole punching 失败尝试 relay
4. 对方离线时使用 mailbox，如果用户启用
5. 所有传输内容保持端到端加密
```

---

## 14. 离线消息设计

### 14.1 无服务器现实

如果对方离线：

```text
无法立即投递消息
```

---

### 14.2 Outbox

本地保存待发送消息。

```rust
pub struct OutboxItem {
    pub id: String,
    pub target_user_id: UserId,
    pub target_device_id: Option<DeviceId>,
    pub encrypted_packet: Vec<u8>,
    pub retry_count: u32,
    pub next_retry_at: u64,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}
```

---

### 14.3 发送策略

```text
1. 对方在线则立即发送
2. 对方离线则进入 outbox
3. 定期尝试连接
4. 超过 expires_at 后标记 Expired
```

---

## 15. 本地存储设计

### 15.1 Web

Web 端使用：

```text
IndexedDB
```

敏感数据需在写入前加密。

---

### 15.2 Native

Native 端使用：

```text
SQLite + SQLCipher
```

数据库密钥：

```text
db_key = HKDF(identity_seed, "lm-talk.storage-key.v1")
```

---

### 15.3 核心表

#### identity

```sql
CREATE TABLE identity (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  user_id TEXT NOT NULL,
  encrypted_identity_seed BLOB NOT NULL,
  kdf_params TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
```

#### devices

```sql
CREATE TABLE devices (
  device_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  device_public_key BLOB NOT NULL,
  device_cert BLOB NOT NULL,
  is_self INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);
```

#### contacts

```sql
CREATE TABLE contacts (
  user_id TEXT PRIMARY KEY,
  display_name TEXT,
  identity_public_key BLOB NOT NULL,
  contact_card BLOB NOT NULL,
  state TEXT NOT NULL,
  trust_level TEXT NOT NULL,
  added_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

#### friend_requests

```sql
CREATE TABLE friend_requests (
  request_id TEXT PRIMARY KEY,
  peer_user_id TEXT NOT NULL,
  direction TEXT NOT NULL,
  status TEXT NOT NULL,
  request_payload BLOB NOT NULL,
  response_payload BLOB,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

#### messages

```sql
CREATE TABLE messages (
  message_id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  sender_user_id TEXT NOT NULL,
  receiver_user_id TEXT,
  group_id TEXT,
  direction TEXT NOT NULL,
  kind TEXT NOT NULL,
  ciphertext BLOB,
  plaintext_encrypted BLOB,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  received_at INTEGER,
  sent_at INTEGER,
  delivered_at INTEGER,
  read_at INTEGER
);
```

#### groups

```sql
CREATE TABLE groups (
  group_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  epoch INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

#### group_members

```sql
CREATE TABLE group_members (
  group_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  status TEXT NOT NULL,
  local_relation TEXT NOT NULL,
  joined_at INTEGER,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (group_id, user_id)
);
```

#### local_blocks

```sql
CREATE TABLE local_blocks (
  user_id TEXT PRIMARY KEY,
  reason TEXT,
  created_at INTEGER NOT NULL
);
```

#### outbox

```sql
CREATE TABLE outbox (
  id TEXT PRIMARY KEY,
  target_user_id TEXT NOT NULL,
  target_device_id TEXT,
  encrypted_packet BLOB NOT NULL,
  retry_count INTEGER NOT NULL,
  next_retry_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER
);
```

---

## 16. 协议编码与签名

### 16.1 所有协议对象必须有版本

每个协议对象包含：

```text
type
version
created_at
expires_at 可选
```

---

### 16.2 不对普通 JSON 直接签名

JSON 字段顺序不稳定，不适合作为签名输入。

签名必须基于确定性编码：

```text
canonical binary encoding
```

建议使用：

- postcard
- canonical CBOR

二维码、调试、人类可读导出可以使用 JSON，但签名输入必须使用确定性编码。

---

## 17. 安全设计

### 17.1 KDF

身份备份包加密使用：

```text
Argon2id
```

建议参数：

```text
memory_kib = 65536
iterations = 3
parallelism = 1
salt = random 32 bytes
```

---

### 17.2 AEAD

推荐：

```text
XChaCha20-Poly1305
```

---

### 17.3 密钥派生

```text
identity_seed
  ↓ HKDF
identity_signing_seed
identity_dh_seed
storage_key
profile_key
```

---

### 17.4 随机数

Native：

```text
OS RNG
```

Web：

```text
crypto.getRandomValues
```

Rust WASM 需要正确配置 `getrandom` 的 JS 支持。

---

### 17.5 敏感数据处理

要求：

- 使用 `zeroize`
- 使用 `secrecy`
- 不在日志中打印私钥
- 不在 Debug 输出敏感字段
- release 模式关闭敏感日志

---

## 18. 隐私边界

LM Talk 保护：

- 消息内容
- 本地数据库
- 好友公钥真实性
- 网络监听下的明文内容

LM Talk 不完全保护：

- IP 地址
- 在线时间
- 通信频率
- 消息大小
- 谁和谁连接
- DHT 查询痕迹
- 设备被入侵后的数据
- 用户截图和转发

产品文案必须明确：

```text
端到端加密保护内容，但不等于完全匿名。
```

---

## 19. Web 端安全边界

纯 Web 版存在代码供应链风险：

```text
如果网页从远程服务器加载，服务器可以更新恶意 JS/WASM。
```

因此：

```text
Web 版适合 MVP 和轻量使用。
高安全版本建议提供 Tauri 桌面端或移动端本地应用。
```

Web 正式版建议：

- PWA 离线缓存
- 构建产物 hash 校验
- 可下载固定版本
- 开源可复现构建

---

## 20. 不做的功能

MVP 不做：

- 公开社区
- 公开群搜索
- 热门群推荐
- 全局账号注册
- 服务器消息存储
- 服务器内容审核
- 全局封禁
- 大群 MLS
- 完整 Signal Double Ratchet
- 自动 DHT 发现
- TURN 中继
- 多设备同步
- 文件传输

---

## 21. MVP 范围

### 21.1 MVP 0：Rust Core

当前已实现：

- Identity create / restore、Identity backup package、UserID 生成、提示词 normalize。
- DeviceID / DeviceCert / DeviceRevoke 基础对象。
- Contact Card 导入导出、Friend Request / Response。
- DirectEnvelope MVP 消息加解密。
- X3DH PreKey Bundle、Signed PreKey、One-time PreKey、Initial Message、shared secret 派生。
- Double Ratchet session state、message key、skipped key、DH step、`x3dh-double-ratchet-v1` envelope。
- GroupInvite、GroupEvent、GroupPolicyState、Sender Key Distribution、Sender Envelope。
- FileManifest、FileChunkEnvelope、文件 hash 校验。
- LocalSafetyPolicy、本地拉黑/过滤、Outbox、MemoryStore、协议对象大小限制。
- 固定测试向量和 core e2e secure flow。

仍需补齐：

- 生产级 Store trait + SQLite/SQLCipher 实现。
- Ratchet replay/window/skipped-key 上限策略最终化。
- 多设备同步、设备撤销事件自动分发。
- 属性测试、模糊测试、外部安全审计。

---

### 21.2 MVP 1：Web Demo

实现：

- 创建身份
- 导出身份备份包
- 导入身份备份包
- 显示 UserID
- 导出 Contact Card
- 导入 Contact Card
- 发送好友请求文本
- 接受好友请求文本
- 加密/解密文本消息
- IndexedDB 本地存储

---

### 21.3 MVP 2：WebRTC 手动连接

实现：

- 手动 offer/answer
- WebRTC DataChannel
- 好友之间在线发送消息
- ACK
- 本地 outbox

---

### 21.4 MVP 3：小群

实现：

- 创建小群
- 邀请好友
- 对方确认入群
- 群消息逐个加密发送
- 本地屏蔽群成员

---

### 21.5 MVP 4：Native Node

当前已实现：

- `lm_node` 控制面 scaffold。
- Public Peer announce 生成、验签、导入、closest 查询。
- Kademlia NodeId、XOR distance、bucket、closest peer 排序；DHT record key/value scaffold 已覆盖 Public Peer、PreKey、Mailbox hint 三类记录 key，带 TTL、republish_at、closest record 查询和过期清理；控制面提供 `POST /dht/record`、`GET /dht/record`、`GET /dht/closest`，snapshot 可保存/合并 DHT records；已定义 `DhtRpcRequest` / `DhtRpcResponse` 并提供本地 `FindNode` / `FindValue` / `StoreRecord` handler，控制面 `POST /dht/rpc` 可作为传输层接入前的 RPC 兼容入口；已提供 due-for-republish 的 closest-k replication plan 和 256 个 bucket refresh target plan，并通过 `GET /dht/replication-plan` / `GET /dht/routing-refresh-plan` 暴露给控制面；`serve-control` 同步周期后会对已配置 control peers 执行 `StoreRecord` replication scaffold，并执行 bounded `FindNode` routing refresh scaffold；replication factor、FindNode limit 和每轮 refresh target 上限可由 config/CLI/env 配置；refresh runner 会合并从已配置 control peers 返回的非过期、node_id 与 peer_id 匹配且非本机的 `RoutingPeer`，用于 bootstrap/control-peer 信任边界内的 routing table 扩展。
- Mailbox：`/mailbox/push`、`/mailbox/take`、`/mailbox/ack`。
- PreKey：`/prekey/publish` 接收 bundle 与 `signed_one_time_prekey_record_texts[]`，`/prekey/get` 可返回 `selected_signed_one_time_prekey_record_text` 并在 `consume=true` 时精确记录 one-time prekey 消费；`/prekey/status` 返回 remaining/low watermark 与 `replenishment_required`；低水位补货由客户端持有 private prekey 后重新发布，节点返回 `replenishment_actor="client"` / `node_generates_user_keys=false` 并且不生成用户密钥；bundle 过期会清理，signed prekey 轮换会重置消费记录与旧 signed OTK records。
- Snapshot：`/sync/snapshot`、`/sync/import`，可粗粒度同步 peers/mailbox/prekeys/signed one-time-prekey records。
- 自动 snapshot sync：`serve-control --config-file node.json` 可加载 JSON 配置，`docs/NODE_CONFIG.md` 记录 schema，`docs/examples/lm-node.config.example.json` 提供样例；control/sync token 支持 CLI、环境变量或 secret 文件；`--sync-peer http://host:port --sync-interval-seconds N` 定时拉取并 merge peer snapshot；`--sync-peer-token`/`--sync-peer-token-file` 可拉取受 token 保护的 peer；`--sync-max-backoff-seconds` 控制失败指数退避；`/sync/status` 暴露 attempts/successes/failures/last_success_at/last_error/next_attempt_at。
- 控制面基础安全与观测：未配置 token 时非 health API 仅允许 loopback；`--control-token` 要求 `Authorization: Bearer ...`；`--cors-allow-origin` 限制浏览器 Origin；`--rate-limit-window-seconds` / `--rate-limit-max-requests` 对非 health API 做 per-client IP 基础限流，超限返回 `429 Too Many Requests`；`GET /control/stats` 暴露 JSON 格式 started_at、请求总数、2xx/4xx/5xx、unauthorized、CORS 拒绝、限流命中、snapshot import/export 次数与字节数、DHT replication runner 运行/records/attempts/successes/failures/last run、routing refresh runner 运行/targets/attempts/successes/failures/nodes_returned/nodes_merged/last run、过期清理运行次数/移除记录数，以及 endpoint 维度请求数、状态码分布、累计/最大耗时等运行指标；`GET /control/metrics` 导出 OpenMetrics 文本，便于 Prometheus 类系统采集。
- `serve-control --state-file` 可保存/恢复节点状态；保存时写入同目录临时文件、fsync 后 rename，降低进程崩溃导致状态文件截断的风险；HTTP e2e 已覆盖 mailbox push 后崩溃、take 未 ack 后崩溃、ack 后崩溃三种恢复语义。
- 节点 e2e：PreKey 同步 + Mailbox 携带 ratchet envelope + 接收方解密。

仍需补齐：

- SQLCipher 或其他加密数据库；当前 SQLite state_db 未做数据库级加密，高敏部署应依赖磁盘加密或后续 SQLCipher。
- 真正 DHT 节点发现、传输层 RPC 执行、远端记录复制和定时 routing table refresh。
- 后续将 control-peer DHT replication scaffold 升级为按 closest-k target 选择远端，并为开放传输层返回节点补充可携带 identity public key 的端到端签名校验。
- WebRTC signaling、relay/TURN 替代能力。
- 更细粒度反滥用策略、真正 DHT 网络安全边界、Relay/TURN 替代能力。

---

## 22. Rust 核心 API 草案

### 22.1 Identity

```rust
impl Identity {
    pub fn create_with_passphrase(
        passphrase: &str,
    ) -> Result<(Identity, IdentityBackupPackage)>;

    pub fn restore_from_backup(
        backup: &IdentityBackupPackage,
        passphrase: &str,
    ) -> Result<Identity>;

    pub fn user_id(&self) -> &UserId;

    pub fn export_contact_card(
        &self,
        display_name: Option<String>,
        device_cert: DeviceCert,
    ) -> Result<ContactCard>;
}
```

---

### 22.2 Contact

```rust
impl ContactCard {
    pub fn verify(&self) -> Result<()>;

    pub fn fingerprint(&self) -> String;
}

pub fn import_contact_card(card: ContactCard) -> Result<Contact>;
```

---

### 22.3 Friend

```rust
impl FriendService {
    pub fn create_request(
        &self,
        from: &Identity,
        to: &Contact,
        note: Option<String>,
    ) -> Result<FriendRequest>;

    pub fn accept_request(
        &self,
        identity: &Identity,
        request: FriendRequest,
    ) -> Result<FriendResponse>;

    pub fn apply_response(
        &self,
        response: FriendResponse,
    ) -> Result<()>;
}
```

---

### 22.4 Message

```rust
pub trait SessionCrypto {
    fn encrypt(
        &mut self,
        plaintext: &[u8],
    ) -> Result<EncryptedEnvelope>;

    fn decrypt(
        &mut self,
        envelope: &EncryptedEnvelope,
    ) -> Result<Vec<u8>>;
}
```

---

### 22.5 Group

```rust
impl GroupService {
    pub fn create_group(
        &mut self,
        name: String,
        members: Vec<Contact>,
    ) -> Result<CreateGroupResult>;

    pub fn create_invite(
        &mut self,
        group_id: &GroupId,
        member: Contact,
    ) -> Result<GroupInvite>;

    pub fn encrypt_group_message_fanout(
        &mut self,
        group_id: &GroupId,
        body: MessageBody,
    ) -> Result<Vec<OutgoingDirectPacket>>;
}
```

---

## 23. 关键风险

### 23.1 身份备份包丢失

如果用户设备丢失且没有身份备份包：

```text
无法恢复身份。
```

---

### 23.2 提示词泄露

如果攻击者同时拿到：

```text
身份备份包 + 提示词
```

则可以接管身份。

---

### 23.3 无服务器可达性

对方离线或 NAT 穿透失败时：

```text
消息无法即时到达。
```

---

### 23.4 Web 代码供应链风险

远程 Web App 更新可能破坏端到端安全。

---

### 23.5 无管理员内容治理

系统不承诺全局过滤违法信息，只提供本地自治工具。

---

## 24. 用户文案建议

### 24.1 创建身份

```text
你的身份由一个随机身份密钥决定。

请设置提示词，并保存身份备份包。
以后换设备时，需要同时拥有：
1. 身份备份包
2. 提示词

丢失任一项，可能无法恢复身份。
```

---

### 24.2 好友请求

```text
对方请求添加你为好友。

请确认对方 UserID 和安全码。
接受后，对方可以与你直接通信。
```

---

### 24.3 陌生人请求

```text
这是陌生人请求。
在你接受前，对方不能直接向你发送附件或群邀请。
```

---

### 24.4 P2P 连接失败

```text
无法连接到对方设备。
可能原因：
1. 对方离线
2. 网络限制
3. NAT 穿透失败

消息已加入待发送队列。
```

---

## 25. 最终设计总结

```text
身份：
  随机 identity_seed 决定 UserID。
  提示词只用于加密 identity_seed。
  恢复身份必须导入身份备份包并输入提示词。

设备：
  UserID 代表人。
  DeviceID 代表设备。
  每台设备有独立设备密钥。

好友：
  Contact Card 导入身份。
  Friend Request / Friend Response 双向确认。
  默认只有 Friend 可以直接通信。

消息：
  端到端加密。
  MVP 使用 X25519 + HKDF + XChaCha20-Poly1305。
  后续升级 X3DH + Double Ratchet。

网络：
  Web MVP 使用手动 WebRTC signaling。
  Native 后续加入 DHT、mDNS、自动发现和 Public Peer。

Public Peer：
  有公网 IP 的节点可作为 bootstrap / DHT / signaling / relay / mailbox。
  Public Peer 不注册账号，不决定好友，不保存明文，不掌握密钥。

群聊：
  MVP 小群逐个加密发送。
  无公开社区，无管理员治理。
  群状态是本地视图。

安全：
  默认好友白名单。
  陌生人请求箱。
  本地拉黑。
  本地过滤。
  不做全局审核和全局封禁。

存储：
  Web 使用 IndexedDB。
  Native 使用 SQLCipher。
  数据只存本地。
```



## 附录：当前端到端测试覆盖

当前 `scripts/test.sh all` / `scripts/test.sh e2e` 覆盖：

- `cargo fmt --check`、workspace Rust 单元测试和 doc-test。
- `lm_core` 身份、Contact Card、好友请求/确认、大小限制、测试向量、DirectEnvelope、X3DH、Double Ratchet、群 Sender Key、文件包、Outbox、MemoryStore。
- core e2e：两用户好友流程、X3DH shared secret、Double Ratchet 双向消息。
- `lm_node`：Public Peer announce、Kademlia closest、Mailbox push/take/ack、PreKey bundle + signed one-time-prekey records publish/get、snapshot roundtrip/import、serve-control 自动 snapshot sync。
- node e2e：节点 PreKey/signed OTK 同步 + Mailbox 携带 ratchet envelope + 接收方解密。
- HTTP control flow：真实 `serve-control` 进程之间同步 PreKey/Mailbox，并覆盖 config-file、token/CORS 基础安全、控制面基础限流、`/control/stats` JSON 指标和 `/control/metrics` OpenMetrics 导出。
- `lm_wasm` smoke：身份、联系人、好友、消息、PreKey/X3DH、Ratchet、群、文件、Public Peer、Mailbox、Signaling。

仍需补齐：proptest/fuzz、跨语言/跨平台测试向量、真实网络延迟/丢包/重连/压力测试、持久化崩溃恢复测试。


## 附录：群聊 Sender Key 实验路径

已新增 `lm-group-sender-key-v1` 与 `lm-group-sender-envelope-v1`：

- 每个群成员可以为某个群生成自己的 Sender Key chain。
- Sender Key Distribution 由发送者身份签名。
- 群消息使用 Sender Key chain 派生 message key，经 XChaCha20-Poly1305 加密。
- Web 群聊在本群存在自己的 Sender Key 时优先生成 Sender Envelope，否则回退 pairwise fanout。

限制：当前 Sender Key 仍需手动分发 distribution；成员变更后的 key rotation、管理员权限、MLS 树更新还未实现。


## 附录：群权限状态机

已新增 `GroupPolicyState`：

- 创建者默认为 admin。
- Rename/AddMember/PromoteAdmin/DemoteAdmin 需要 admin。
- RemoveMember 允许 admin 踢人，也允许成员自己退出。
- 防止只剩一个 admin 时把唯一 admin 移除。
- Web 应用群事件时优先通过 WASM policy state 校验并更新成员/admin/sequence。

后续仍需：更细粒度邀请策略、审计 UI、成员变更触发 Sender Key rotation。


## 附录：Sender Key 轮换策略

当前实现新增基础轮换：

- `GroupPolicyState::event_requires_sender_key_rotation` 标记 AddMember/RemoveMember 需要轮换。
- Web 应用成员变更群事件后，会移除该群已保存的其他成员 Sender Key。
- 如果当前用户仍在群内，会自动为自己生成新的 Sender Key Distribution，并提示分发。
- 如果当前用户被移出群，则清除本群 Sender Key。

限制：新的 Sender Key Distribution 仍需手动发给现有成员；尚未实现自动按成员 fanout 分发，也未实现 MLS 树更新。


## 附录：Sender Key Distribution 自动 fanout

Web 现已支持 Sender Key Distribution fanout：

- 创建或轮换本群 Sender Key 后，自动为当前群成员生成 pairwise 加密分发包。
- 分发包载荷前缀为 `lm-group-sender-key-message-v1:`，收到后会自动导入对应 sender 的 Sender Key。
- 成员变更触发本端 Sender Key rotation 后，也会自动生成新的 distribution fanout。

限制：fanout 仍需要通过复制/WebRTC/Mailbox 发送；尚未在协议层做 MLS 风格树分发。


## 附录：one-time prekey 精确消费

已新增基础精确消费：

- `PreKeyBundle::new_with_signed_one_time_prekey_records` 可生成不把 OTK 列表签进 bundle 的公开 bundle，并为每个 OTK 生成独立 `SignedOneTimePreKeyRecord`。
- `x3dh_initiator_secret_with_one_time_prekey_record` 可用选中的 signed OTK record 派生 shared secret；旧的 `x3dh_initiator_secret_with_one_time_prekey_id` 仍用于兼容 bundle 内 OTK。
- `lm_node /prekey/publish` 接收 `signed_one_time_prekey_record_texts[]`；`/prekey/get?consume=true` 不删除整个 bundle，而是记录已消费的 one-time key id，并优先返回未消费的 `selected_signed_one_time_prekey_record_text`。
- 响应返回 `selected_one_time_prekey_id`、`selected_signed_one_time_prekey_record_text`、`consumed_one_time_prekey_ids`、`remaining_one_time_prekeys`、`signed_one_time_prekey_records`、`low_one_time_prekeys`、`replenishment_required`、`replenishment_actor` 和 `node_generates_user_keys`；`/prekey/status` 可不拉取 bundle 只查询补货状态。
- Web 拉取/领取 PreKey 后会优先保存 selected signed OTK record，并在创建 X3DH initial message 时使用该 record；没有 signed record 时回退旧 selected id。

限制：节点只提示低水位/补货需求，不代替客户端生成私钥补货；多设备补货同步、signed prekey 轮换广播和真正 DHT 复制仍需完善。
