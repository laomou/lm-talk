# 协议稳定性

本文档定义了 LM Talk 在生产就绪发布前协议对象的兼容契约。它并未冻结每个实现细节；而是标记哪些线格式已稳定到可互操作，哪些仍属过渡性，以及如何引入更改。

## 稳定性级别

- **Stable**：可被独立部署的客户端/节点依赖。破坏性更改需要新的类型/版本/前缀和迁移指南。
- **Transitional**：当前客户端可用，但仍可兼容性更改。破坏性更改需明确发布说明和降级/互操作指南。
- **Internal/debug**：不是公共兼容面。不要跨发布依赖它。

## 版本规则

1. 签名对象必须在签名/规范字段中包含 `type` 和 `version`。
2. 导出文本前缀属于线格式，不能为不兼容 schema 重用。
3. 不兼容更改应采用新 `type` 或前缀，而非静默重新解释。
4. 阅读器应拒绝不支持的 `version` 值，除非已记录为前向兼容。
5. 大小限制属于 DoS 边界，仅可在迁移说明下变得更严格。
6. 过期字段在存在/对象类型要求时必须由验证器检查。

## 稳定线对象

以下对象计划在发布候选互操作中保持稳定：

| 对象 | 前缀 / 类型 | 稳定性 | 说明 |
| --- | --- | --- | --- |
| 身份备份 | `lm-identity-backup-v1:` | Stable | 受口令保护的身份种子包。 |
| Contact Card | `lm-contact-card-v1:` | Stable | 签名身份、X25519 密钥、显示名和设备证书。DHT `ContactCard` 记录使用该 payload。 |
| 好友请求 | `lm-friend-request-v1:` | Stable | 签名请求，检查过期。 |
| 好友响应 | `lm-friend-response-v1:` | Stable | 签名响应，绑定请求/联系人。 |
| 旧版 DirectEnvelope | `lm-direct-envelope-v1:` / `x25519-static-hkdf-xchacha20poly1305-v1` | Transitional | 保持兼容；严格 E2EE 应优先 Ratchet + 每设备封闭槽。 |
| 文件包 | `lm-file-package-v1:` | Stable | 加密文件清单/分片；文件名策略为本地处理。 |
| 设备证书 | `lm-device-cert-v1` | Stable | 由身份密钥签名；包含设备签名公钥和设备箱公钥。 |
| 设备撤销 | `lm-device-revoke-v1:` | Stable | 由身份密钥签名；客户端应停止信任撤销的设备 ID。 |
| 消息回执 | `lm-message-receipt-v1:` | Stable | 针对消息 ID 和投递 ID 的送达/已读回执。 |
| Mailbox 消息 | `lm-mailbox-message-v1:` | Stable | 节点在存储前验证发送者签名和 TTL。 |
| 公共节点公告 | `lm-public-peer-announce-v1:` | Stable | DHT `PublicPeer` 记录使用该 payload。 |
| PreKey bundle | `lm-prekey-bundle-v1:` | Stable | DHT `PreKey` 记录使用该 payload。 |
| 签名一次性 PreKey 记录 | `lm-signed-one-time-prekey-v1:` | Stable | 用于可补货的一次性密钥。 |
| MailboxHint DHT 值 | URL/string 值 | Transitional | 当前记录值为地址字符串；将来可能由签名 hint 对象替代。 |

## 过渡/活跃开发对象

| 对象 | 类型 / 前缀 | 稳定性 | 需注意 |
| --- | --- | --- | --- |
| Double Ratchet 状态 | `lm-ratchet-state-v1:` | Transitional | 本地/导出状态，不是公共网络消息。更改需本地迁移。 |
| Ratchet 信封 | `x3dh-double-ratchet-v1` | Transitional | 当前 Web 路径用于 secure session；协议冻结前保持兼容。 |
| Secure session offer/response | `lm-secure-session-offer-v1`, `lm-secure-session-response-v1` | Transitional | Mailbox 运送的设置辅助。 |
| 群组发送密钥分发 | `lm-group-sender-key-v1:` / payload 前缀 | Transitional | 成员/轮换策略仍在完善。 |
| 群事件 | `lm-group-event-v1:` | Transitional | 策略已足够演示，仍需外部审查。 |
| 每设备信封 | `lm-per-device-envelope-v1` | Transitional | 支持封闭槽；占位/回退仅兼容性用途，严格模式应阻断。 |
| 自同步包 | `lm-self-sync-v1` | Transitional | 签名轻量同用户状态同步；不是消息历史同步协议。 |
| 自同步请求 | `lm-self-sync-request-v1` | Transitional | 最近自同步包的缺口修复请求/响应。 |
| 全数据备份 | `lm-data-backup-v1:` | Transitional | 用于同一身份的加密备份/恢复格式。 |
| 节点状态文件 | `lm-node-state-file-v1:` | Internal/operator | 原生节点本地持久化格式；不要视为联邦网络协议。 |

## DHT 记录类型

当前节点 DHT 记录类型和键命名空间为该发布冻结：

| 类型 | 键命名空间 | 值格式 | 验证 |
| --- | --- | --- | --- |
| `PublicPeer` | `public-peer` over `peer_id` | `lm-public-peer-announce-v1:` 导出文本 | PublicPeer 签名、过期、键/peer 匹配。 |
| `PreKey` | `prekey` over `user_id` | `lm-prekey-bundle-v1:` 导出文本 | PreKey bundle 签名、过期、键/用户匹配。 |
| `MailboxHint` | `mailbox-hint` over `user_id` | 地址字符串 | 客户端接受非空、大小受限、可接受的 URL/multiaddr/mailbox 模式。 |
| `ContactCard` | `contact-card` over `user_id` | `lm-contact-card-v1:` 导出文本 | ContactCard 签名、可选过期、键/用户匹配。 |

添加 DHT 记录类型需新的命名空间、验证规则、最大大小审查、测试向量覆盖、发布说明和发现 UI/诊断更新。

## Mailbox 消息类型

当前发布的 Mailbox 类型冻结为：

| 类型 | 典型负载 | 说明 |
| --- | --- | --- |
| `SignalOffer` | WebRTC/session offer 文本 | 安全会话辅助传输。 |
| `SignalAnswer` | WebRTC/session answer 文本 | 安全会话辅助传输。 |
| `DirectEnvelope` | 直接/Ratchet/每设备信封负载 | 主要直接消息投递路径。 |
| `GroupFanout` | 面向接收者的群组信封/fanout 负载 | 群组投递路径。 |
| `DeliveryReceipt` | `lm-message-receipt-v1:` 已送达 | 送达状态协调。 |
| `ReadReceipt` | `lm-message-receipt-v1:` 已读 | 已读状态协调。 |
| `Other` | 通过前缀/JSON `type` 检查类型的负载 | 兼容性桶，用于联系人更新、设备撤销、安全会话 JSON、数据备份、自同步和未来过渡性负载。 |

Web 将更高级别的本地发件箱类型（如 `contact-update`、`device-revoke`、`self-sync` 和数据备份）映射到 Mailbox `Other`；接收方必须检查负载类型/前缀以识别这些高级对象。添加 Mailbox 类型需节点验证、Web 映射、向后兼容的 `Other` 回退指南和旧节点的发布说明。

## 设备与 ContactCard 更新策略

1. 新设备应创建包含 `device_box_public_key` 的 `lm-device-cert-v1`。
2. Contact Card 重新导出时应按 `device_id` 保留已知有效设备证书。
3. Contact Card 更新应通过 Mailbox `contact-update`、同用户自同步和 DHT `ContactCard` 记录分发。
4. 接收者在合并 Contact Cards 时必须保留本地信任状态：指纹验证、撤销、阻止状态和已读回执策略。
5. 设备撤销优先于过时 Contact Card 设备列表。
6. 严格 E2EE 模式应要求已验证联系人和封闭每设备槽进行发送/接收。

## PreKey 轮换策略

1. 当缺失、过期或一次性密钥不足时，可重新发布 PreKey bundle。
2. 签名一次性 PreKey 记录至多在节点状态中消费一次，消费状态必须快照/同步。
3. DHT `PreKey` 记录必须验证 bundle 签名和键命名空间。
4. 客户端应优先使用可用的签名一次性 PreKey 记录，然后回退到可重用签名 prekey 行为。
5. 未来不兼容 PreKey 更改需采用新 bundle 前缀/类型。

## 错误兼容性

当前用户可见错误尚未成为稳定的数字错误代码 API。在错误代码冻结前：

- 协议验证器应在内部返回精确类型错误。
- 节点 HTTP 端点应继续使用稳定状态类：`400` 无效输入、`401` 未授权、`413` 载荷过大、`429` 限流、`5xx` 服务器失败。
- 发布候选必须记录任何 Web UI 依赖的错误文本更改。

## 错误文本依赖

协议尚未公开稳定的数字应用错误代码。以下文本依赖项已知，并在更改前必须审查：

| 领域 | 依赖 | 当前保护 |
| --- | --- | --- |
| `state_db` 加密 | 测试和发布证据查找 `encryption_mode is plain`、`SQLCipher provider requested`、以及 `state_db_encrypted` 指标。 | `lm_node` 测试和 SQLCipher smoke 脚本。 |
| `state_file` 加密 | 测试期望缺少口令和明文现有 state file 时出现 fail-closed 消息。 | `lm_node` 测试。 |
| DHT 发现分类 | Web 根据包含 `过期`、`验签`、`签名`、`signature`、`未找到`、`not found`、`record`、`格式`、`invalid`、`超时`、`timeout`、`failed to fetch` 的错误进行分类。 | Web 诊断/退避逻辑。 |
| 严格 E2EE / 封闭槽 | Web 面向用户的错误提示包含缺少已验证指纹、缺少活动设备、缺少 `device_box_public_key`、非封闭入站槽和每设备信封签名失败。 | Web E2E 和手动策略流。 |
| Mailbox 回执 | Contact Card 更新 ACK 和消息回执流程依赖 `lm-message-receipt-v1:` 解析和送达/已读类型字符串。 | 核心/Web 测试。 |

更改这些字符串仅在相应测试更新和发布说明到位时允许。未来稳定发布应用结构化错误代码替换基于文本的匹配。

## 废弃策略

- 旧版 DirectEnvelope 和占位/回退每设备槽为兼容路径。
- 新的严格部署应启用封闭槽发送/接收和已验证联系人策略。
- 未来生产发布可能在至少一个发布周期后发出默认警告或禁用回退路径，并附迁移说明。

## 生产冻结检查清单

在将协议稳定性标记为生产发布前：

- [ ] 外部审计审查了上述 Stable 和 Transitional 对象。
- [x] 稳定签名/加密对象的测试向量已存在（见 `docs/TEST_VECTOR_COVERAGE.md`）。
- [x] DHT 记录类型列表和键派生命名空间已冻结。
- [x] Mailbox 类型映射和 `Other` 回退行为已记录。
- [x] ContactCard/DeviceCert 合并和撤销策略已有互操作测试。
- [x] PreKey 轮换/消费策略已有互操作测试。
- [x] Web 和节点测试中的错误代码/文本依赖已记录。
- [ ] 发布证据索引链接了 fuzz、联邦、SQLCipher 和审计产物。
