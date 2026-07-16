# LM Talk 遗留事项 / TODO

版本：v0.1  
日期：2026-07-14  
状态：实现同步草案

本文档记录 `docs/DESIGN.md` 中尚未完全细化的设计决策、协议细节、实现前置任务和少量长期边界。

优先级定义：

- **P0**：开始核心编码前必须明确，否则容易返工或产生安全问题。
- **P1**：MVP 阶段需要明确。
- **P2**：正式版或后续增强需要明确。

---

## 当前实现状态快照（2026-07-14）

已完成或基本成型：

- `lm_core`：身份/备份、Contact Card、好友请求/响应、DirectEnvelope、X3DH PreKey、Double Ratchet、群 Sender Key、群权限状态、文件分片加密包、本地安全策略、Outbox、MemoryStore、大小限制、属性测试、跨平台测试向量、核心导入解析 malformed fuzz-like 覆盖。
- `lm_wasm`：大部分 core 能力已导出，并有 smoke 测试。
- `lm_node`：HTTP control plane、Public Peer announce、Kademlia ID/distance/closest scaffold、DHT record key/value scaffold 与控制面 store/find/closest、DHT RPC 消息/本地处理 scaffold 与 `POST /dht/rpc` 入口、closest-k replication plan 与 routing refresh target plan 及控制面 plan 端点、control-peer StoreRecord replication runner、Mailbox push/take/ack、Mailbox TTL/配额/message_id 去重、PreKey publish/get、独立 signed one-time prekey records 发布/同步/消费、PreKey 过期清理/轮换重置/低水位提示、snapshot sync/import、serve-control 定时 snapshot sync、控制面 token/CORS 基础安全、控制面 previous token 轮换窗口、控制面 per-client IP 基础限流、`/control/stats` JSON 运行指标、`/control/metrics` OpenMetrics 文本导出、过期清理维护统计、状态文件原子保存。
- 测试：`scripts/test.sh all` 当前通过 Rust 测试、core/node e2e、HTTP control flow、WASM smoke、Web build/e2e；Web 侧补齐了 IndexedDB 持久化和 Web RNG 生成身份的真实流程验证。

关键边界：

- `lm_node` 仍是控制面 + snapshot sync scaffold，不是真正生产 DHT。
- Mailbox/PreKey 可支撑 demo；Mailbox 已有基础 TTL/配额/message_id 去重、take 分页、ack 批量限制与拒绝统计、delivery 状态查询和 ACK tombstone 持久化、控制面 per-client IP 限流和 SQLite state_db 持久化，但仍缺完整客户端状态合并、多设备回执同步与更强反滥用。
- Core 协议对象已可测，属性测试和跨平台测试向量已补齐；Double Ratchet replay、乱序 skipped-key 消费和 skip-window 边界已有属性测试；常见导入文本/附件 JSON 解析已补 malformed 输入不崩溃和超限拒绝覆盖；已新增 cargo-fuzz/libFuzzer harness 脚手架。仍需长时间 fuzz 运行、持续语料回归、AFL/独立安全测试和外部安全审计。
- 本地持久化仍偏 Web IndexedDB / MemoryStore；Native node 已有 SQLite state_db，SQLCipher/客户端完整数据加密仍未实现。

---

## 当前未完成功能清单（2026-07-15 更新）

> 当前 `lm_core` / `lm_wasm` / `lm_node` 已具备可测试 MVP scaffold；Web 产品化流程仍是最直接的用户可用性缺口。下面按当前代码状态整理真实缺口。

### 已有基础路径但需产品化

1. **正式网络设置区产品化**
   - Web 已有 `lm_node 控制面 URL`、启用/停用节点、连接状态和 IndexedDB 持久化。
   - 设置页已展示同步服务数量、主节点和已配置令牌数量。
   - 设置页已说明多节点按顺序尝试，成功节点会自动置顶为主节点。
   - 设置页已默认折叠控制面原始多行/JSON 状态，只展示首行摘要。
   - Web 保存/启用同步服务前会校验每行 URL，并显示带行号的错误提示。
   - 设置页同步服务摘要会提示非本机远端缺令牌数量。
   - 设置页检测到同步服务令牌时会提示令牌只保存在本机浏览器，诊断报告默认不导出。
   - 设置页默认以脱敏列表展示同步服务，只有点击编辑时才显示原始地址/令牌输入框。
   - 设置页仅在编辑地址/令牌时显示保存按钮，减少隐藏令牌状态下的误操作。
   - 设置页远端缺令牌条目会提示通过编辑地址并追加 `|令牌` 修复。
   - token 管理仍需更产品化。

2. **PreKey 自动发布/补货产品化**
   - Web 已支持自动生成并发布 PreKey Bundle 与 signed one-time-prekey records。
   - 本地 Private PreKey Bundle 已加密持久化，节点不生成用户密钥。
   - 设置页已有 PreKey 发布入口、状态刷新和剩余 one-time key / 低水位补货摘要。
   - 设置页已有低水位检查补货入口，发现节点提示补货时会重新生成并发布客户端持有的 PreKey。
   - 登录后和手动同步会先检查节点 PreKey 状态，缺记录时自动发布，低水位时自动补货。
   - 设置页已展示 PreKey 自动检查/发布/补货状态和最近失败原因。
   - 设置页在 PreKey 自动检查/发布/补货失败后提供手动重试入口。
   - 设置页 PreKey 摘要已展示 selected one-time key 和 signed record 状态，减少查看原始 JSON 的需求。
   - 设置页已提供清除 PreKey 公开原文入口，可清空节点返回 JSON、公开 PreKey Bundle、signed OTK、解析 JSON 和 selected record 临时文本，同时保留本地加密 private bundle 用于补货。
   - 设置页已提供清除安全会话原文入口，可清空 Offer/Response 输入输出文本且不删除已建立会话。
   - 仍需进一步减少高级 JSON 暴露。

3. **添加好友后的自动安全建链产品化**
   - Web 已能从节点拉取并消费对方 PreKey，创建 X3DH Initial Message / Double Ratchet 初始状态，并通过 Mailbox 发安全会话响应。
   - 接受好友请求后会自动通过 Mailbox 发送安全会话 Offer；对方收取后会应用 Offer 并回传 Response。
   - 聊天头和联系人详情已显示端到端会话状态，缺失会话时可手动本地建链。
   - 聊天头和联系人详情已显示安全建链失败原因，成功发送 Offer、应用 Offer/Response 或本地重建会话后会清除旧错误。
   - 安全建链失败后可从聊天头或联系人详情手动重试发送 Offer。
   - 安全建链失败提示已支持手动清除。
   - 安全会话 Offer 发送失败后会进入 outbox，使用现有定时重试器自动重发。
   - 安全会话 Offer 失败入队会复用相同联系人/载荷的未发送 outbox 项，避免重复堆积。
   - 聊天头和联系人详情已显示最近安全建链尝试时间，便于判断自动/手动重试是否触发。
   - 聊天头和联系人详情已显示最近安全建链成功时间，便于区分最近重试与最终成功状态。
   - 聊天头和联系人详情已显示连续安全建链失败次数，成功或手动清除后归零。
   - 聊天头和联系人详情已显示安全建链待重试 outbox 数量，避免重复点击重试。
   - 仍需更细粒度建链重试状态和去重策略。
   - 没有节点或没有 PreKey 时，继续支持复制粘贴安全会话流程。

4. **离线消息 Mailbox 路径增强**
   - 单聊和群 fanout 已能在 WebRTC 不可用且节点启用时通过 `/mailbox/push` 投递，并维护 `queued` / `sent` / `mailbox` / `failed` 状态。
   - Web 已有 outbox 定时调度器、当前联系人重发、全部队列重发、取消发送、基础失败分类和端到端送达回执基础路径。
   - 节点已提供 `/mailbox/status`，可查询 delivery 是否尚未被取走、已取未 ACK 或已 ACK/过期/不存在，并返回该用户 mailbox 消息摘要、bytes 用量和 per-user bytes 配额，用于客户端区分“对方未取”和“ACK/状态合并待恢复”，也便于展示 mailbox 配额压力；`/mailbox/push`、`/mailbox/take` 和 `/mailbox/ack` 响应也会返回 pending bytes / per-user bytes quota；Web 收件箱已展示最近一次 Mailbox 容量用量/配额，并在达到 80%/100% 时提示接近上限或已达上限；Web 设置页的节点健康摘要也会展示 /health 返回的 peers、PreKey、Mailbox 用量和配额，并展示 /sync/status 中 DHT peer 失败/隔离摘要；设置页提供 DHT key 派生（含我的 PreKey/MailboxHint/PublicPeer 快捷填充和一键 kind/value FindValue 查找）、“发布并查 DHT”、一键发布并检查 PreKey/MailboxHint/PublicPeer 全部 DHT、“刷新节点健康”、单 peer“重置”、“查找 DHT 记录”、“运行 DHT 维护”、“复制 DHT 记录”和“刷新 DHT 路由”入口，便于排查去中心化节点健康、将 UserID/peer_id 转为 DHT key、解除误隔离/退避并手动触发 FindValue/maintenance/replication/routing refresh；FindValue 摘要会展示命中 record kind 和 key 前缀，并会把命中的 PreKey/MailboxHint/PublicPeer value 回填到对应 Web 状态；PreKey/PublicPeer 回填前会尝试验签/解析，MailboxHint 会校验地址前缀、回填匹配联系人的 mailbox_hint_url，并在发送时优先尝试该联系人的 HTTP(S) MailboxHint、失败后回退同步服务；也可一键加入同步服务（已存在时不重复添加），并会拒绝 key 不匹配或已过期的 DHT record，避免无效 DHT 记录静默进入可用状态；诊断报告已纳入 DHT 查找/复制/路由刷新、DHT 操作历史与 peer health 摘要；DHT 操作历史随 IndexedDB/完整数据备份持久化和恢复，可单独复制/导出/导入合并给运维排障，导入支持历史 JSON、诊断报告 JSON 或字符串数组，会过滤诊断脱敏/截断占位并限制单条/总量，导入前会显示数量/去重/保留上限提示并二次确认，清空前也会二次确认，以避免误改或误删排障证据。
   - `lm_core` 已新增签名 `MessageReceipt`（`Delivered` / `Read`）协议对象，可通过 Mailbox/WebRTC 作为端到端送达/已读回执载荷，并有导入、验签、篡改和 fuzz-like malformed 输入覆盖；`lm_wasm` 已导出创建/验签 MessageReceipt 的 API，并支持 `delivery-receipt` / `read-receipt` MailboxMessage kind；Web 已在收到 direct mailbox/WebRTC 消息后自动生成签名 Delivered receipt；当前会话可见且用户开启“当前会话自动发送已读回执”时还会自动生成 Read receipt；并能处理 signed delivery/read receipt 更新本地消息状态为已送达/已读，保留旧 `lm-delivery-ack-v1` 兼容；全局开关会随完整数据备份导入/导出持久化，聊天页还提供每联系人策略（跟随全局/始终开启/关闭）。Web 发送方会保存 Mailbox `delivery_id`，同步时通过 `/mailbox/status` 恢复 `pending` / `delivered_unacked` / `acked` 状态，用于回执丢失时把已取/已 ACK 的消息推进到“已送达”；本地消息状态同时记录 `delivered_at` / `read_at` 时间戳并在聊天气泡展示；完整数据备份/多设备导入合并会按 `protocol_message_id` 合并同一消息的更高回执状态和时间戳，便于排查和未来多设备状态合并。
   - 仍需完善 Read receipt 的更多产品策略（例如每联系人策略）、多设备回执同步、更复杂的 ack 丢失恢复、跨设备状态合并规范和更细的失败恢复策略。

5. **Mailbox 收取与处理产品化**
   - Web 已能登录后/切回页面/手动同步时调用 `/mailbox/take?limit=N` 分页收取，自动处理 direct-envelope、好友请求/响应、群 fanout、文件包和安全会话 offer/response，成功后 `/mailbox/ack`，并在 `more=true` 时继续拉取后续页；达到单次分页上限仍有更多内容时会提示再次同步。
   - 通讯录已有正式收件箱入口，展示好友请求、群邀请、最近 Mailbox 处理摘要、分页收取摘要和最近失败原因。
   - 收件箱已按失败原因归类展示 Mailbox 处理失败摘要。
   - 收件箱已保留最近 Mailbox 处理失败队列，支持手动重试和清空失败项。
   - 收件箱去重摘要已显示本地去重记录最新/最旧处理时间，便于判断裁剪范围。
   - 仍需长期 dedupe 保留和跨设备策略。

6. **好友请求 Mailbox UI 收口**
   - 好友请求和接受/拒绝响应已可作为 Mailbox `Other` 载荷投递和处理。
   - 好友请求已纳入通讯录收件箱。
   - Web 已有好友请求/好友通过通知和 RequestSent 状态手动重发入口。
   - 好友请求发送失败会保留最近错误，并在聊天提示和联系人详情中展示，重发时清除旧错误。
   - 好友请求失败提示已支持手动清除。
   - 仍需更完整的跨节点失败恢复。

### P0：让 Web 页面像聊天软件一样可用

7. **群聊正式收发流程**
   - 群消息 fanout 自动对每个成员发送：WebRTC 在线直发，否则 Mailbox。
   - 群邀请、群事件、Sender Key Distribution 自动进入收件箱并应用。
   - Web 已在群聊头显示非好友、被拉黑、缺少联系人和 Sender Key 回退提示。
   - 群详情页已显示群事件 sequence、管理员数量和最近群事件摘要。
   - 群详情页已展示最近群事件应用失败原因，并支持清除错误提示。
   - 仍需更细的冲突/乱序/权限失败恢复 UI。

8. **文件发送走正式流程**
   - 文件包生成后可自动通过 WebRTC 或 Mailbox 发送。
   - 收到文件包后显示附件卡片，用户点击后再解密/下载。
   - Mailbox 收到文件包后不自动解密，只提示待解密状态。
   - 保持“不自动下载陌生附件”。

9. **正式页面信息架构整理**
   - 左侧：身份摘要、网络状态、联系人、群组。
   - 右侧：聊天头、消息列表、输入框、附件按钮。
   - 把协议 JSON 面板移到“高级/调试”区域，默认不展开。
   - 设置页同步状态的原始多行/JSON 输出已默认折叠。
   - 诊断报告 JSON 预览已默认隐藏，只在用户显式点击后展开。
   - 不加入摄像头扫码功能；二维码只生成和复制原文。

10. **本地数据安全增强**
    - Web IndexedDB 已有应用层加密路径，规格见 `docs/STORAGE_SPEC.md`。
    - 消息、联系人、群、outbox、ratchet 会话、好友请求、群邀请、群 Sender Key、Mailbox 失败队列和同步服务地址/令牌等敏感字段已纳入应用层加密；测试会检查 IndexedDB 不直接出现聊天明文、联系人名片/显示名和 sync token URL。
    - 分表加载已支持单条损坏记录隔离：联系人/消息/群/outbox/ratchet 等记录解密失败时跳过坏记录、加载其余数据并提示用户恢复。
    - 身份提示词重加密后会用新提示词重新派生本地存储密钥并重写 IndexedDB；Web E2E 验证同步服务配置仍加密且可用新提示词重新登录恢复。
    - 删除本地身份会同步清理该 user_id 前缀下的 IndexedDB 分表数据，避免只删登录入口但遗留本机加密聊天数据。
    - 旧 localStorage 迁移到 IndexedDB 分表时，会在新分表写入成功后再删除原始状态；Web E2E 已覆盖迁移后恢复同步设置且原始状态被清理。
    - 继续补完整迁移回滚、异常提示和更系统的明文字段审计。
    - 定期检查只保留必要索引明文。

### P1：可用性与可靠性

1. **Outbox 重试机制**
   - 定时重试 WebRTC / Mailbox 投递。
   - 指数退避、最大重试次数、过期时间、当前聊天取消发送和基础失败分类已具备实现。
   - 当前聊天头已显示最近 Outbox 失败原因。
   - 设置页已有待发送队列摘要、失败原因和清理已发送入口。
   - 通讯录已有正式收件箱入口，展示好友请求、群邀请、Mailbox 摘要和最近失败原因。

2. **Mailbox 防重复与去重**
   - Web 已本地记录已处理 delivery_id / message_id，并随 IndexedDB meta 持久化。
   - 重复拉取会跳过重复处理并继续 ack 对应 delivery。
   - 通讯录收件箱已展示本地去重记录数量，并支持清空本地去重记录。
   - Web 已为本地去重记录保留处理时间，按 30 天 / 1000 条裁剪并兼容旧字符串记录。
   - 仍需跨设备 dedupe 合并策略。

3. **节点同步自动化**
   - [x] `serve-control --sync-peer http://host:port --sync-interval-seconds N` 可定时拉取 peer snapshot 并 merge。
   - [x] 支持多个 sync peer 的持久配置文件和失败退避：`sync_peers[]`、`sync_max_backoff_seconds`、`/sync/status`。
   - [x] 后续替换为真正传输层 DHT 查询/复制；Native Node 已接入 libp2p request-response DHT 查询/复制 scaffold。

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
   - Web 版供应链风险提示已存在，设置页已展示 PWA Service Worker / 缓存状态和 Web 包版本。
   - Web 静态入口已加入 CSP meta 和 no-referrer 策略，限制脚本/对象/frame/form 等默认能力，同时保留 WASM、PWA、blob/data 预览和用户配置同步节点连接能力；Web E2E 覆盖 CSP/referrer meta 存在。
   - Service Worker 注册 URL 和缓存名已带构建版本，避免不同构建共用同一个离线缓存。
   - 设置页已展示当前 `lm-talk-pwa-*` 固定缓存名，便于确认离线包对应构建版本。
   - 固定版本离线包已具备静态资源缓存优先策略，首次在线后可按构建版本离线使用。

### 功能设计缺口

#### P0

1. **身份备份 UX 闭环**
   - 注册/导入页已有不可恢复提示；注册成功页已有复制/下载身份备份、备份校验码；导入页会显示粘贴文本校验码；本地身份删除前已有保存确认提示。
   - 注册成功后已有“验证导入”入口，可将刚导出的身份文本带入导入页让用户用提示词重新导入验证。
   - Web 设置页已支持用新提示词重加密当前身份备份包，并同步更新本机保存的登录入口。

2. **消息 / Mailbox / Outbox 统一状态机**
   - 定义同一消息经 WebRTC、Mailbox、Outbox 多路径发送时的状态合并规则。
   - Web 已有基础失败分类：网络失败、节点拒绝、载荷过大、请求过频、已过期、联系人不可用。
   - Web 外发消息气泡已显示短 protocol message_id，便于排查重复消息和送达回执。
   - Web 已将 Mailbox 状态显示为“已投递节点，待对方收取”，与送达回执“已送达”区分。
   - Web 外发 Mailbox 消息气泡已显示等待对方收取时长，用于判断“对方未取”状态。
   - Web 收到重复 direct mailbox 消息时会重新发送送达回执，缓解对端回执丢失后的重投递。
   - Web Mailbox 同步状态会显示重复消息补发回执数量。
   - Web 对送达回执、设备撤销、文件包和安全会话 Offer 等系统 outbox 载荷做相同联系人/相同载荷去重，避免失败重试重复堆积。
   - Web 待发送队列已显示每条 outbox 的下次重试时间或可立即重试状态。
   - Web 当前聊天头已拆分展示待发送和失败 outbox 数量。
   - Web 已有 Outbox 定时重试、当前联系人手动重发、取消发送和清理已发送队列；仍需对方未取、解密失败、重复消息、ack 丢失恢复和状态合并规范。

3. **同步与通知策略**
   - Web 已有登录后自动发布/收取、页面切回自动收取和手动“立即同步”。
   - 页面切回自动收取已有 30 秒节流；手动“立即同步”不受节流影响。
   - 设置页已提供自动发布 PreKey、自动收取 Mailbox 和自动同步节点快照开关。
   - 设置页已展示同步触发优先级：手动同步、登录后自动、前台恢复节流、Outbox 定时重试和节点快照定时同步。
   - 设置页已合并展示 PreKey、Mailbox、Outbox 和节点快照的同步失败摘要。
   - 设置页已提供“恢复同步失败”，串联 PreKey 重试、Mailbox 失败队列、Outbox 重试和节点快照自动同步恢复。
   - 设置页已展示跨触发器恢复结果分项，包括 Mailbox 成功/失败、Outbox 触发/剩余和节点快照重试。
   - 设置页已保留最近 5 次跨触发器恢复历史，超出后自动丢弃最旧条目。
   - 设置页已支持按关键词筛选跨触发器恢复历史。
   - 跨触发器恢复历史已随本地状态持久化，并可导出 JSON。
   - 跨触发器恢复历史已支持清空。
   - Web 已有基础通知权限入口、页面后台消息/文件/好友请求/好友通过/群邀请/安全会话通知和同步失败通知。
   - 设置页已展示在线/离线、前台/后台和低电量状态，用于解释自动同步限制。
   - 设置页已展示通知权限、前后台通知行为、自动收取开关和浏览器后台暂停限制。
   - 设置页已展示 Push API、Background Sync 和 Periodic Background Sync 能力探测，用于解释 PWA 后台能力边界。
   - Service Worker 已处理 `sync` / `periodicsync` / `push` 后台事件，但出于端到端加密约束只通知用户打开应用完成同步，不在后台读取密钥或消息；设置页提供“注册后台同步”入口，会尝试注册 one-shot Background Sync 和 Periodic Sync，并展示/持久化最近收到的后台事件标签历史，Web E2E 覆盖该安全提示路径。
   - 仍需接入真实 Push 订阅服务器、平台级 periodic sync 授权策略和更完整后台任务 telemetry。

#### P1

4. **群聊生命周期**
   - Web 已有群创建、邀请、删除本地群、群名变更、成员增删、管理员升降和群事件应用基础路径。
   - Web 已有本设备退出群聊语义：确认后删除本地群、群消息和群 Sender Key，不通知其他成员。
   - Web 已在群详情页展示最近群事件权限/乱序/应用失败原因。
   - Web 收到移除当前身份的群事件后，会在本地群视图提示已被移出并阻止继续发送群消息。
   - Web 生成群管理事件前会提示非管理员/已移出群聊的权限失败，并记录到群详情最近事件错误。
   - Web 已对群事件重复/过期/乱序显示恢复建议，提示同步缺失事件或清除旧事件错误。
   - Web 群详情已提供“通知退群”，会生成成员自己的 RemoveMember 事件并 fanout 后删除本地群。
   - Web 群详情已区分“通知退群”和“仅本机退出”，避免误以为本地删除会通知其他成员。
   - Web 群事件摘要已将 RemoveMember 显示为“成员退出”，不再使用移除成员文案。
   - Web 收到本地不存在的群事件时会提示可能尚未接受邀请或已仅本机退出。
   - Web 群详情已显示本地群视图更新时间，群事件/错误/Sender Key 异常使用完整日期时间。
   - 仍需设计成员自己退出后的双方视图和本地群视图冲突处理。
   - Web 已在群详情和聊天顶部展示 Sender Key 缺少 distribution、解密失败和轮换后分发失败。
   - 创建或手动生成 Sender Key Distribution 后会自动通过 WebRTC/Mailbox 分发，失败时进入 outbox 自动重试。
   - 成员变更触发本端 Sender Key rotation 后，会自动投递新的 Distribution fanout 并在群详情记录分发失败。
   - 明确历史消息策略：新人默认不可见、手动转发、重新加密范围。

5. **文件传输体验**
   - Web 已有文件包生成、正式聊天附件入口、WebRTC/Mailbox/outbox 投递、接收后解密和下载基础路径。
   - Web 已显示文件名、MIME、大小和危险文件类型警告。
   - 发送可执行/安装脚本等危险扩展名附件前会弹出确认，避免误发送高风险文件；Web E2E 已覆盖取消路径。
   - 收到危险扩展名附件时，点击解密生成下载链接前也会要求用户确认来源可信。
   - Web 已有收到文件附件卡片和下载入口。
   - Web 已支持取消已选择/已生成的文件包。
   - Web 已在生成文件包前提示浏览器存储空间不足风险。
   - Web 已显示文件读取、加密封装、投递、入队、完成、失败等阶段状态。
   - Web 已对收到的图片附件提供内联预览。
   - Web 已对非图片附件显示类型标签，并明确不内联预览、需下载后用本机应用打开。
   - Web 已在待发送队列展示文件/消息 outbox 过期时间。
   - Web 已显示文件读取阶段的字节级进度和封装完成状态。
   - Web 已在 WebRTC/Mailbox/outbox 投递阶段显示加密包大小和完成/失败状态。
   - Web 已在收到待解密文件包时展示文件名、MIME、明文大小和加密包大小。
   - Web 点击下载已解密文件时会更新附件阶段和状态文本。
   - Web 文件消息气泡会在点击下载后显示已下载时间。
   - 聊天附件区已显示当前联系人文件发送失败原因，并可直接触发 outbox 重试。
   - Web 已明确当前文件发送边界：最大 16 MB，Web 端整包发送，暂不支持断点续传。
   - 仍需更细粒度上传/下载传输进度和断点恢复。

6. **本地数据管理**
   - Web 已有清空当前会话、删除好友、删除群聊、清理本地身份、清理浏览器缓存、清理已发送 outbox、存储占用展示和诊断页摘要。
   - Web 已有聊天列表/通讯录搜索和当前会话消息搜索；搜索只在本机解密后的内存数据上执行，不落明文索引。
   - Web 导入完整数据备份前会提示覆盖当前本地联系人、群聊、消息和待发送队列。
   - 设计完整数据备份恢复后的冲突处理。

#### P2

7. **反滥用 UX**
   - Web 已有本地拉黑、文本过滤、陌生人请求收件箱、批量忽略好友请求和批量拉黑请求来源。
   - 设置页已有本地过滤器开关、级别、外部链接/可执行文件提示和高风险入站丢弃配置。
   - 发送包含外部链接或可执行/脚本文件名等风险文本前会弹出本地确认；取消不会清空输入框，也不会擅自改写用户将发送给对方的文本。
   - Web 已将同一来源的重复未处理好友请求隔离到垃圾请求区，并支持恢复或清空。
   - 收件箱已显示陌生请求频率摘要；同一来源短时间重复请求会进入垃圾请求区，支持单条或全部恢复误拦截。
   - Web 已按来源记录 24 小时好友请求计数，超过本地阈值后自动隔离到垃圾请求区，并支持清空本地计数。

8. **诊断报告规范**
   - Web 已有诊断页和状态摘要报告，并声明不导出提示词、身份私钥或消息明文。
   - 允许字段、禁止字段、`diagnostics_version` 和分享前预览确认已整理到 `docs/DIAGNOSTICS_SPEC.md`。
   - Web 已支持可选脱敏账号摘要和同步服务地址。
   - Web 已支持只生成并复制摘要报告。
   - 诊断报告同步区已导出 token_count / missing_remote_token_count 这类非敏感计数，不导出令牌内容。
   - 诊断报告和诊断页最近日志会先脱敏再截断单行长文本，降低 token、备份包和完整协议载荷误入报告的风险。
   - Web E2E 已覆盖 Bearer token、URL token 和身份备份包前缀不会进入诊断 JSON。

9. **可访问性与国际化**
   - Web 已有 `zh-CN` 页面语言、按钮焦点样式、toast `aria-live`、主导航/搜索/消息列表/弹窗基础可访问性语义。
   - Web 已为登录/注册/导入、添加联系人、建群、显示名和同步服务等关键表单控件补充显式 label / aria-label。
   - Web 已为本地身份删除、诊断入口、缓存清理和退出登录等紧凑操作按钮补充 aria-label。
   - Web 聊天消息、聊天列表、群事件摘要、诊断日志和 Outbox 过期时间已使用 `zh-CN` 格式。
   - Web 安全建链最近尝试时间已使用完整 `zh-CN` 日期时间，避免跨天排查歧义。
   - Web 文件消息气泡已使用统一文件大小格式，而不是 raw bytes。
   - Web 名片、消息、Envelope、Signal 和文件包等 UI 超限错误已使用统一文件大小格式。
   - 仍需系统检查键盘可用性、屏幕阅读器朗读顺序和颜色对比度。
   - 仍需统一错误文案、更多日期/文件大小显示和语言包边界。

### P2：协议与长期增强

1. **生产级 DHT / Kademlia 网络**
   - 当前 `lm_node` 已有 HTTP control-plane DHT RPC、libp2p request-response DHT RPC、bootstrap discovery、closest-k replication 和 routing refresh scaffold。
   - 已补基础本地容量、单条记录大小限制、TTL 上限和 record key-kind-value 语义校验；仍需生产级查询鲁棒性、跨公网运维、Sybil/垃圾记录防护、传输安全策略和压力测试。

2. **Relay / TURN 替代能力**
   - 有公网 IP 的节点可选做 bootstrap / DHT / relay / mailbox。
   - Relay 不能成为强中心依赖。

3. **MLS 或更完整群聊协议**
   - 当前群聊是 Sender Key / fanout 实验路径。
   - 大群、成员变更 epoch、历史策略还需完整设计。

4. **生产级身份备份**
   - Web 当前存在 wasm-local 可用性路径；生产要重新做浏览器安全加密备份。
   - 支持改提示词、重新导出、备份完整性校验。

5. **安全审计与测试增强**
   - 固定协议测试向量、属性测试、跨平台一致性测试、浏览器真实流程 E2E 已建立。
   - 已补核心导入解析的 fuzz-like malformed 输入属性测试，覆盖 Contact/Friend/Backup/PreKey/Signal/DHT/Mailbox/Group/Ratchet/File/Device revoke 等文本入口；附件 Manifest、Chunk JSON、Chunk ciphertext decode 前检查和 Device revoke import 也有显式长度上限。
   - 已新增 `fuzz/` cargo-fuzz/libFuzzer harness：`core_imports`、`node_dht_rpc`、`node_control_request`；运行方式见 `docs/FUZZING.md`。
   - 已新增 `scripts/release-check.sh`、`scripts/fuzz-smoke.sh` 和 `docs/RELEASE_CHECKLIST.md`，将 fmt、Rust core/node、node e2e、fuzz harness compile check、可选 fuzz smoke、Web build/e2e 串成发布候选自动化门禁；GitHub Actions CI 会运行 `release-check quick`，新增 `dependency-audit` job 执行 `cargo audit` 与 runtime/build-toolchain `npm audit`，并在 PR 上运行 `dependency-review` 阻止高危依赖变更。
   - 已新增 tag 触发的 native node release workflow，会为 Linux x86_64、macOS Intel/Apple Silicon、Windows x86_64 构建 `lm_node` 归档并发布 SHA256/构建信息；仍需补 macOS notarization、Windows Authenticode/code signing 和签名 provenance。
   - 已新增仓库级 `SECURITY.md`，说明私下漏洞报告流程、敏感材料脱敏要求和已知非生产阻塞项。
   - 继续补长时间 fuzz 运行记录、持续语料回归、AFL/独立 fuzz、真实网络压力测试和外部安全审计。

---

## P0：核心编码前必须明确

### 1. 提示词规范

提示词归一化规则已迁移到 `docs/IDENTITY_SPEC.md`，并由测试向量覆盖。这里仅保留产品策略层面的待决策项。

待决策：

- 是否允许用户自定义提示词。
- 是否默认生成随机词提示词。
- 最低长度或最低强度。
- 是否允许中文、英文、符号混合。
- 是否做弱密码检测。
- 错误提示如何展示。

协议规范：见 `docs/IDENTITY_SPEC.md`。

---

### 2. 身份备份包产品策略

身份备份包格式已迁移到 `docs/BACKUP_SPEC.md`。这里仅保留导入/导出产品策略。

待决策：

- 是否支持二维码。
- 是否支持复制粘贴文本形式。
- 备份包损坏和提示词错误是否区分提示。
- Web `wasm-local` 备份路径是否需要迁移到生产级浏览器 Argon2id 或其他 KDF。

协议规范：见 `docs/BACKUP_SPEC.md`。

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

core 已实现主要协议对象大小限制。这里保留 UI 和未来文件协议的产品化事项。

待完成：

- 超限时错误码。
- Web UI 如何提示。
- 后续文件传输是否走单独协议。

---

### 8. 测试向量维护

固定测试向量已建立，并由 core/wasm 测试覆盖。这里保留后续维护要求。

当前已有：

- identity_seed -> UserID
- passphrase + backup -> identity_seed
- Contact Card 签名与验签
- Friend Request 签名与验签
- 消息加密与解密
- 篡改密文失败

后续需要：

- 新协议版本增加测试向量。
- Ratchet replay/window/skipped-key 不变量增加测试向量或属性测试。
- 跨语言实现出现后增加兼容性验证。

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

基础联系人名片格式见 `docs/CONTACT_SPEC.md`。

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

信任等级枚举已迁移到 `docs/CONTACT_SPEC.md`。

待定义：

- 各等级 UI 如何显示。
- 升级信任等级流程。
- 是否允许用户手动标记已验证。

---

### 13. 安全指纹格式

基础指纹方案已迁移到 `docs/CONTACT_SPEC.md`。

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

已明确：

- 新人默认看不到历史。
- Web 群邀请卡片和接受后的群事件摘要会提示历史消息不会自动同步。

待定义：

- 是否允许邀请者手动转发最近 N 条。
- 转发历史是否重新加密。

建议：

```text
新人默认看不到历史。
历史同步必须由老成员手动选择。
```

---

### 18. 本地过滤器 MVP 范围

本地过滤只覆盖端侧风险提示和本地阻断。

MVP 建议只做：

- 陌生人附件禁止
- 附件不自动下载
- 外部链接警告
- 可执行文件警告
- 本地拉黑

暂不做：

- 内置敏感词库
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
- X3DH / signed prekey / 独立 signed one-time prekey 协议对象已完成；已接入 lm_node 控制面发布/领取/消费，仍需真正 DHT 发布/查询/复制。
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
- 独立 `lm-signed-one-time-prekey-v1` records，可由身份签名、验签、导出/导入。
- private prekey bundle 本地保存格式。
- X3DH initial message。
- 发起方/响应方 shared secret 派生测试。

当前状态与剩余事项：

- prekey bundle 与 signed one-time-prekey records 可发布到 `lm_node` 控制面，并可通过 snapshot / SQLite state_db 保存和粗粒度同步。
- 节点已有 HTTP control-plane 和 libp2p request-response DHT RPC scaffold；仍需把 PreKey 查询/复制接入正式产品路径，并补 DHT 抢占/垃圾记录防护。
- one-time prekey 消费记录已实现；独立 signed one-time-prekey records 已接入节点发布/消费路径，还需多设备同步与补货协调。
- 复制粘贴版 Ratchet DH public key 交换和 UX 串联已完成；Web 可从 lm_node 拉取 PreKey 与 selected signed OTK record，还需自动节点发现。
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

Web/WASM 已有完整数据备份导入/导出路径：用当前身份备份包和提示词派生身份后，加密导出本地持久化状态；设置页已有生成、显示/粘贴、下载、导入合并和导入覆盖入口。这里保留生产级策略和 UX 待办。

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

待完成：

- 备份版本迁移策略。
- 大数据量分片或流式导出。
- 更细粒度逐项冲突选择、下载/导入 UX 和失败恢复。
- 是否把节点配置、processed mailbox ids、诊断日志纳入数据备份。

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
   - [x] `serve-control --state-file` 采用同目录临时文件 + fsync + rename 的原子保存，避免普通写入在崩溃时截断主状态文件。
   - [x] 为 mailbox deliveries、prekey bundles、consumed one-time prekeys、public peers 增加 SQLite 正式存储：`--state-db` / `state_db`。
   - [x] 保留 snapshot import/export 作为迁移和调试能力。
   - [x] 增加 state-file 崩溃恢复测试：push 后崩溃、take 未 ack 后崩溃、ack 后崩溃。

2. **Mailbox 生命周期**
   - [x] TTL 过期清理（push/take/restore/merge 路径会清理过期 delivery）。
   - [x] 基础 per-user / per-node quota（`max_mailbox_messages_per_user` / `max_mailbox_bytes_per_user` / `max_mailbox_bytes`），并已提供 config/CLI/env 配置入口。
   - [x] 基础 message_id 去重；delivery_id 去重保留在 snapshot merge 路径。
   - [x] 持久化 quota/TTL/去重索引到正式数据库：SQLite `mailbox_deliveries` 表包含 `expires_at`、`to_user_id`、`message_id` 唯一索引。
   - [x] state-file crash recovery 覆盖 mailbox push 后崩溃、take 未 ack 后崩溃、ack 后崩溃。
   - [x] 控制面基础 per-client IP 限流：`--rate-limit-window-seconds` / `--rate-limit-max-requests`，超限返回 `429`，`/health` 不计入限流。
   - [x] 控制面入站客户端连接配置读写超时，避免慢连接长时间占用单线程控制面处理循环；相关超时参数通过 `/health` 暴露。
   - [x] 控制面超大请求体返回 `413 Payload Too Large`，超大 header 返回 `431 Request Header Fields Too Large`，错误路径真实 HTTP status 与统计值一致，避免全部混入 `400` 难以排查。
   - [x] 控制面 HTTP parser 拒绝超大 method/path/header line、冲突重复 `Content-Length` 和非空 `Transfer-Encoding`，降低请求走私/解析歧义和超长首行消耗；真实 `serve-control` HTTP 集成测试覆盖 413/431/400 实际响应。
   - [x] 控制面响应附加 no-store、nosniff、no-referrer、Permissions-Policy 和受限 CSP 等安全响应头，避免浏览器缓存或误用敏感控制面响应。
   - [x] Mailbox `push` 支持全局限流：`mailbox_global_rate_limit_window_seconds` / `mailbox_global_rate_limit_max_messages`，默认关闭，超限返回 `429`。
   - [x] Mailbox `push` 支持按 sender UserID 限流：`mailbox_sender_rate_limit_window_seconds` / `mailbox_sender_rate_limit_max_messages`，默认关闭，超限返回 `429`。
   - [x] Mailbox `push` 异常 payload / 拒绝原因统计：`maintenance.mailbox_push_rejects` 与 OpenMetrics `lm_node_mailbox_push_rejections_total{reason=...}`。
   - [x] `take` 不删除，只有处理成功后 `ack` 删除；已覆盖 state-file 与 SQLite `state_db` 下 push 后崩溃、take 未 ack 后崩溃、ack 后崩溃的重启重试语义。
   - [x] Mailbox `take` 支持 `limit` 分页领取并返回 `returned` / `pending` / `more`，默认限制单次返回数量，避免大邮箱一次性响应过大。
   - [x] Mailbox `ack` 限制单次 delivery id 数量和单个 id 长度，并使用集合匹配删除，避免超大 ack 请求造成过多 CPU/内存消耗。
   - [x] Mailbox `ack` 异常 payload / 拒绝原因统计：`maintenance.mailbox_ack_rejects` 与 OpenMetrics `lm_node_mailbox_ack_rejections_total{reason=...}`，覆盖 invalid_json、invalid_user_id、too_many_ids、empty_id 和 id_too_large。
   - [x] `GET /mailbox/status` 返回 per-user summary 与单个 delivery 的 `pending` / `delivered_unacked` / `acked` / `absent_or_expired` 状态，并将 ACK receipt/tombstone 纳入 snapshot 与 SQLite state_db 持久化，为“对方未取”“已取未 ACK”和“ACK 已完成”的客户端恢复逻辑提供节点端基础。
   - [x] 增加节点 Mailbox 压力/故障自动化测试：批量写入 120 条、分页 take、部分 ACK、查询 acked/delivered_unacked 状态，并通过 snapshot 恢复验证 ACK tombstone 与未 ACK delivery 均保留。
   - [x] `/health` 暴露 Mailbox take 默认上限、ack delivery id 数量上限、单个 id 长度上限、Mailbox 总字节/每用户字节/每用户消息数配额、控制面入站/peer 超时和 control-peer 响应大小上限，便于部署确认反滥用参数。

3. **PreKey 生命周期**
   - [x] signed prekey 轮换时重置旧 one-time prekey 消费记录。
   - [x] one-time prekey 低水位提示：`remaining_one_time_prekeys` / `low_one_time_prekeys`。
   - [x] bundle 过期清理：restore/get/take/merge 路径会移除过期 bundle 和消费状态。
   - [x] 自动补货仍由客户端持有 private prekey 后重新发布；节点不生成用户密钥：`/prekey/get` / `/prekey/status` 返回 `replenishment_required`、`replenishment_actor="client"`、`node_generates_user_keys=false`。
   - [x] 定义独立 signed one-time-prekey record 协议对象：`SignedOneTimePreKeyRecord` 可由身份签名、验签、导出/导入，避免未来新增 OTK 时必须重签整个 bundle。
   - [x] 将节点 PreKey 存储/发布/消费路径从 bundle 内 one-time list 切换到独立 signed one-time-prekey records，避免 bundle 级签名与消费记录耦合；兼容旧 bundle 内 one_time_prekeys。
   - [x] 节点拒绝超过 core 上限的 signed one-time-prekey record 批量发布，避免 PreKey 发布路径被超量 OTK 记录滥用。

4. **控制面安全**
   - [x] 未配置 token 时，除 `/health` 外只允许 loopback 客户端访问；生产 `scripts/run.sh` 在非 loopback 绑定且未显式提供 control token/token file 时会拒绝启动，并提供 `--check-config` 预检。
   - [x] `--control-token` / `LM_NODE_CONTROL_TOKEN` 支持 Bearer token 保护非 health API。
   - [x] `--cors-allow-origin` / `LM_NODE_CORS_ALLOW_ORIGIN` 支持 CORS 白名单。
   - [x] token 可从配置文件、CLI 或环境变量加载。
   - [x] `--control-token-file` / `LM_NODE_CONTROL_TOKEN_FILE` 和 config `control_token_file` 支持从 secret 文件加载；Unix 下会拒绝 group/other 权限过宽或 symlink secret 文件，降低 token 泄漏风险。
   - [x] `--rate-limit-window-seconds` / `LM_NODE_RATE_LIMIT_WINDOW_SECONDS` / config `rate_limit_window_seconds` 与 `--rate-limit-max-requests` / `LM_NODE_RATE_LIMIT_MAX_REQUESTS` / config `rate_limit_max_requests` 支持基础限流。
   - [x] token 轮换策略：见 `docs/NODE_CONFIG.md` 的 secret 文件原子替换、滚动更新和验证流程。
   - [x] TLS/反向代理部署说明：见 `docs/NODE_CONFIG.md` 的 Nginx/Caddy 示例和部署检查清单。

### P1：节点自动同步与网络

1. **自动 snapshot sync**
   - [x] CLI 参数配置 peer control URL 列表：`--sync-peer http://a,http://b`。
   - [x] `serve-control` 定时拉取 `/sync/snapshot` 并 merge 到本地节点。
   - [x] 合并 peers/mailbox/prekeys/consumed records 时保持幂等。
   - [x] 增加 `/sync/status`，记录 attempts/successes/failures/last_success_at/last_error/next_attempt_at。
   - [x] `--sync-peer-token` / `LM_NODE_SYNC_PEER_TOKEN` 支持从受 token 保护的 peer 拉取 snapshot。
   - [x] `--sync-peer-token-file` / `LM_NODE_SYNC_PEER_TOKEN_FILE` 和 config `sync_peers[].token_file` 支持从 secret 文件加载，并复用 Unix secret 文件权限校验。
   - [x] 同步失败指数退避：`--sync-max-backoff-seconds`。
   - [x] `serve-control --config-file` / `LM_NODE_CONFIG_FILE` 支持 JSON 配置文件。
   - [x] 敏感字段拆分：control/sync token 可通过环境变量或 secret 文件加载。
   - [x] 配置文件 schema 文档：`docs/NODE_CONFIG.md` 与 `docs/examples/lm-node.config.example.json`。

2. **DHT scaffold 演进**
   - [x] 增加本地 `DhtRecordStore` scaffold：`store` / `find_value` / `closest_records` / `due_for_republish` / `prune_expired`。
   - [x] 为 Public Peer、PreKey record、Mailbox hint 定义 namespaced deterministic record key。
   - [x] 记录包含 TTL、`republish_at`、kind、value；支持 closest-k 本地记录查询和过期清理。
   - [x] 控制面提供 `POST /dht/record`、`GET /dht/record`、`GET /dht/closest`，snapshot 保存/合并 DHT records。
   - [x] 定义 `DhtRpcRequest` / `DhtRpcResponse` 并提供本地 `FindNode` / `FindValue` / `StoreRecord` handler scaffold。
   - [x] 控制面 `POST /dht/rpc` 可执行 DHT RPC 消息，作为传输层接入前的兼容入口。
   - [x] HTTP control-plane DHT RPC client helper 可向远端 `/dht/rpc` 发送 RPC JSON，并复用 peer bearer token；control-peer HTTP 请求配置连接/读写超时且响应读取有大小上限，避免恶意/异常 peer 卡住后台任务或返回超大响应占用内存。
   - [x] serve-control 同步周期后可对 due-for-republish records 向已配置 control peers 执行 `StoreRecord` replication scaffold。
   - [x] `/control/stats` 与 `/control/metrics` 暴露 DHT replication runner 的 runs、records、attempts、successes、failures 和 last run 时间。
   - [x] serve-control 同步周期后可执行 bounded control-peer `FindNode` routing refresh runner scaffold，并统计返回节点数量。
   - [x] routing refresh runner 可合并来自已配置 control peers 的可信返回节点：过滤过期、node_id/peer_id 不匹配、本机节点、FindNode 返回中的自引用/重复节点，以及已知响应 peer id 时未比响应 peer 更接近 refresh target 的节点，并写入 routing table，同时在运行统计/OpenMetrics 中暴露 rejected non-closer/duplicate routing nodes。
   - [x] `RoutingPeer` 返回节点可携带 identity public key；verified merge 路径会校验 public peer announce 签名，snapshot / SQLite state_db 会持久化该字段。
   - [x] DHT record 拒绝统计覆盖控制面 store、DHT RPC StoreRecord、sync snapshot 导入和 FindValue found/closer records，避免查询路径静默丢弃无效记录而无法观测攻击/故障。
   - [x] HTTP/libp2p DHT transport 校验响应体 logical `request_id` 与请求一致，并将 DHT `Error` 响应视为失败，降低异常/恶意 peer 串扰响应被上层误用的风险。
   - [x] DHT FindValue / FindNode 客户端侧对远端返回的 closer records/nodes 做处理上限，即使恶意 peer 忽略请求 limit 返回超量对象，也只进行有界合并和校验。
   - [x] DHT FindValue runner 已支持有界迭代查询：初始 peer 返回 verified closer nodes 后，会按目标 key XOR distance 排序，并在 `max_peers` 限制内继续查询，减少单跳 scaffold 和恶意返回顺序的影响；控制面 `GET /dht/key?kind=...&value=...` 可派生 public-peer/prekey/mailbox-hint DHT key，`GET /dht/maintenance?factor=N&limit=N&max_targets=N` 可一键运行 replication 与 routing refresh，`GET /dht/replicate?factor=N` 可手动触发 due record replication runner，`GET /dht/routing-refresh?limit=N&max_targets=N` 可手动触发 refresh runner，`GET /dht/find-value?key=<hex>&limit=N&max_peers=N&alpha=N` 已接入该 runner，并支持 `kind/value` 直接派生 key 查询，并有真实 HTTP 双节点 e2e 覆盖；即使关闭自动 snapshot sync（`sync_interval_seconds=0`），仍可复用已配置 sync/control peers 进行手动 DHT 查询；HTTP-control transport 也会把 routing table 中同 scheme `http://` routing peers 加入 runner 候选，但不会向发现的第三方 peer 传播已配置 bearer token，避免 token 泄漏；已补基础终止条件：找到有效 record 后停止继续消耗查询预算；查询响应会返回命中的 record（如有），`/control/stats` 和 `/control/metrics` 已暴露 `query_rounds` / `alpha` / `exhausted` / `peers_quarantined` 以及手动 FindValue attempts/命中/closer 统计，并有真实 HTTP 双节点 e2e 覆盖，且 FindValue runner 已按 `alpha` 在每轮并发发起查询请求，不再只是顺序批处理；当响应 peer 的 peer_id 已知时，FindValue 会拒绝未比响应 peer 更接近目标 key 的 closer nodes，并暴露 `rejected_non_closer` 指标；FindValue 还会拒绝 closer nodes 中的自引用、已查询或已入队重复候选，并暴露 `rejected_duplicate` 指标，降低恶意/低质量 peer 用不前进或重复路由结果消耗查询预算；DHT runner 会根据 `sync_status` 中的 consecutive_failures/failures 对配置 peer 排序，优先健康 peer；replication、routing refresh 和 FindValue 的 DHT RPC 成功/失败也会回写 peer health、退避和连续失败状态；当 FindNode/FindValue 响应只提供不前进或重复的 routing hints 时，也会把响应 peer 记为可疑失败进入退避链路；并跳过 consecutive_failures 达到可配置阈值且仍处于退避期的本轮 quarantine peer，next_attempt_at 到期后重新纳入候选；`POST /sync/peer/reset` 可手动清除单个 peer 当前失败/退避状态，便于误隔离或网络恢复后快速恢复；`/control/metrics` 暴露 `lm_node_dht_peer_quarantined{peer=...}`，并聚合 replication/refresh/find-value 中被 quarantine 跳过的 peer 数，便于持续告警；仍需更完整的生产级查询终止策略、更强 peer scoring 和恶意路由防护；routing peer 合并失败已纳入 `maintenance.routing_peer_rejects` 与 OpenMetrics，便于观察 expired、node_id 不匹配、本机节点、缺少 identity public key、identity key 无效、签名无效、地址数量超限和单条地址过长等异常 closer nodes；routing peer announce 地址数量/长度也已设上限，避免恶意 peer 用超多/超长地址膨胀 routing table、snapshot 和控制面响应。
   - [x] libp2p DHT RPC codec 显式限制 request/response 大小，并收紧 request-response 并发 streams；libp2p swarm 启用 connection-limits（pending/established/total/per-peer），`/health` 暴露这些上限，便于部署确认。
   - [x] DHT runner 参数可通过 config/CLI/env 配置：replication factor、FindNode limit、每轮 refresh target 上限、transport 和 peer quarantine threshold；生产 `scripts/run.sh` 已透传这些 DHT runner 参数并支持 `--check-config` 预检；`--config-file` 模式不会用脚本默认值覆盖配置文件，只有显式参数或环境变量才作为 CLI override。
   - [x] 生成 due-for-republish records 的 closest-k replication plan。
   - [x] 生成 256 个 Kademlia bucket routing refresh target plan。
   - [x] 控制面提供 `GET /dht/replication-plan` 与 `GET /dht/routing-refresh-plan`。
   - [x] DHT runner/helper 通过 `DhtTransport` 抽象发送 `FindNode` / `FindValue` / `StoreRecord` RPC；当前默认实现仍是 HTTP control-plane `/dht/rpc`，为后续 TCP/WebSocket/QUIC 传输接入预留边界。
   - [x] bounded `FindValue` lookup scaffold 可通过 transport 查询 peers，保存命中的 DHT record，并合并返回的 closer records 与 verified closer nodes。
   - [x] 增加 libp2p request-response 协议 scaffold：`/lm-talk/dht-rpc/1` 使用 JSON 编码承载现有 `DhtRpcRequest` / `DhtRpcResponse`。
   - [x] 增加 libp2p TCP/noise/yamux swarm scaffold，可挂载 DHT request-response behaviour 并监听本地地址。
   - [x] libp2p inbound `DhtRpcRequest` 可复用 `NativeNode::handle_dht_rpc` 生成 response。
   - [x] 本地双 libp2p swarm 可通过 request-response 完成 `FindNode` / `FindValue` / `StoreRecord` roundtrip，并复用现有 DHT record/routing 逻辑。
   - [x] 增加 `Libp2pDhtTransport` helper，可通过 `libp2p://<multiaddr>` + `peer_id` 发送真实 request-response `FindNode` / `FindValue` / `StoreRecord` RPC。
   - [x] serve-control DHT runner 可通过 config/CLI/env 选择 `http-control` 或 `libp2p` transport。
   - [x] 增加 `serve-dht-libp2p` 常驻监听入口，可处理 inbound DHT request-response RPC 并持久化 state。
   - [x] `serve-dht-libp2p` 支持配置 bootstrap peers，启动时拨号已知 libp2p DHT 节点作为 discovery seed。
   - [x] libp2p DHT listener 连接 bootstrap peer 后会自动发送 `FindNode` discovery，并合并返回的 verified routing peers。
   - [x] 已配置 control peers 支持按 `sync_peers[].peer_id` 匹配 closest-k target 执行 DHT `StoreRecord` replication；未配置 peer_id 时保持全量 control-peer 兼容行为。
   - [x] 开放传输层 closest-k replication：libp2p transport runner 可复用已发现 routing peers 作为真实网络 RPC 目标。
   - [x] 本地 DHT record store 已有基础容量上限，超出时优先淘汰最早过期/最早创建的记录，避免无界增长。
   - [x] `/health` 暴露 `dht_record_capacity`，便于运维确认当前 DHT record 数量与容量上限。

3. **节点可观测性**
   - [x] 结构化日志：`log_format` / `--log-format` / `LM_NODE_LOG_FORMAT` 支持 `text` 或单行 JSON，覆盖启动、请求访问、sync、DHT runner 和状态保存错误事件。
   - [x] `/health` 暴露 mailbox/prekey/peer 基础数量。
   - [x] `/sync/status` 暴露同步 peer attempts/successes/failures/last_success_at/last_error/next_attempt_at/consecutive_failures。
   - [x] `/control/stats` 暴露控制面 started_at、请求总数、2xx/4xx/5xx、bad request、unauthorized、CORS 拒绝和限流命中次数。
   - [x] `/control/stats` 增加 endpoint 维度请求数、2xx/4xx/5xx、累计耗时、最大耗时和 last_status。
   - [x] `/control/metrics` 导出 OpenMetrics 文本格式，覆盖控制面全局与 endpoint 指标。
   - [x] `/control/stats` / `/control/metrics` 暴露 snapshot import/export 次数与字节数。
   - [x] `/health` / `/control/stats` / `/control/metrics` 暴露过期清理运行次数、mailbox 过期 delivery 数和 prekey 过期 bundle 数。
   - [x] `/control/stats` / `/control/metrics` 暴露 DHT control-peer replication runner 运行、records、attempts、success/failure 和 last run 时间。
   - [x] `/control/stats` / `/control/metrics` 暴露 DHT routing refresh runner 运行、targets、attempts、success/failure、nodes_returned、nodes_merged 和 last run 时间。
   - [x] `/control/metrics` 暴露每个 sync peer 的 attempts、successes、failures、consecutive_failures 和 next_attempt_at，便于部署观察自动同步退避状态。
   - [x] `/control/stats` / `/control/metrics` 暴露后台任务调度延迟：`lm_node_background_schedule_delay_micros_*`。
   - [x] `/control/stats` / `/control/metrics` 暴露持久化 SQLite 数据库页/空间指标：`lm_node_state_db_*`。
   - [x] SQLite `state_db` 连接启用 WAL、`synchronous=FULL`、`busy_timeout=5000` 和 `foreign_keys=ON`，并有单元测试覆盖；Unix 下 `state_db` 主文件和 WAL/SHM sidecar 以及兼容 `state_file` 保存结果会收紧为 `0600`，降低未加密本地状态泄漏风险；`/health`、`/control/stats` 和 `/control/metrics` 明示 `state_db_encrypted=false` / `lm_node_state_db_encrypted 0` 和权限硬化状态，避免部署方误判 SQLCipher 已完成；新增 `state_db_require_encryption` / `--state-db-require-encryption` / `LM_NODE_STATE_DB_REQUIRE_ENCRYPTION` fail-closed 开关，要求数据库级加密时当前 plain SQLite 构建会拒绝启动；`serve-control` 与 `serve-dht-libp2p` 真实进程路径均有覆盖。

### P2：生产网络能力

1. **真正 DHT / Kademlia**
   - 节点发现。
   - routing table refresh。
   - record replication。
   - 更强的 Sybil/垃圾记录防护。

2. **Relay / TURN 替代能力**
   - 允许公网节点作为可选 relay/mailbox/bootstrap。
   - Relay 不得成为明文可见或强中心依赖。

3. **节点部署规范**
   - [x] systemd/container 示例：见 `docs/NODE_CONFIG.md`。
   - [x] 数据备份/恢复：见 `docs/NODE_CONFIG.md`。
   - [x] 升级兼容策略：见 `docs/NODE_CONFIG.md`。

## 法律与产品边界 TODO

需要在用户协议或 README 中明确：

```text
1. 用户对自己发送和接收的内容负责。
2. 软件不托管公开内容。
3. 软件不保证匿名。
4. 软件不保证消息一定送达。
5. 端到端加密保护内容，但不隐藏所有元数据。
6. 无服务器意味着身份丢失后无法由平台找回。
```

---

## 文档结构

当前文档入口是 `docs/README.md`。已拆分：

- 架构与计划：`docs/DESIGN.md`、`docs/MVP_PLAN.md`、`docs/TODO.md`
- 安全与数据：`docs/SECURITY_MODEL.md`、`docs/IDENTITY_SPEC.md`、`docs/BACKUP_SPEC.md`、`docs/STORAGE_SPEC.md`
- 协议规格：`docs/CONTACT_SPEC.md`、`docs/FRIEND_SPEC.md`、`docs/MESSAGE_SPEC.md`、`docs/GROUP_SPEC.md`、`docs/PUBLIC_PEER_SPEC.md`、`docs/NETWORK_SPEC.md`
- 节点部署：`docs/NODE_CONFIG.md`、`docs/examples/lm-node.config.example.json`

后续原则：

- `DESIGN.md` 只保留跨模块总览和实现状态。
- `*_SPEC.md` 维护稳定协议和数据格式。
- 已完成事项从 `TODO.md` 移出，或改为对应文档引用。
- 发布候选检查入口见 `docs/RELEASE_CHECKLIST.md` 与 `scripts/release-check.sh`；通过该脚本仍不等于生产发布完成。

---

## 当前最高优先级清单

建议下一步优先完成：

```text
1. Web 同步设置产品化：多节点、token、连接状态和错误提示。
2. PreKey 自动补货与失败重试：低水位提示、隐藏 JSON 调试细节。
3. 好友通过后自动 X3DH + Double Ratchet 建链：失败时回退复制粘贴流程。
4. Mailbox 产品化：正式收件箱、失败重试、长期去重和送达回执。
5. 本地数据应用层加密增强：完整迁移回滚、更多字段审计。
6. Native node 持久化增强：SQLCipher 或等价数据库加密、更多备份演练和运维指标。
7. 节点自动同步增强：libp2p DHT transport 产品化、更多 control-peer 故障/压力测试。
8. Outbox 调度器：指数退避、取消发送、过期、delivery status。
9. 协议稳定化：错误码、对象大小限制、Contact Card 更新策略、PreKey 轮换策略。
10. 安全测试增强：长时间 libFuzzer/AFL fuzz 运行与语料回归、真实网络压力测试、外部安全审计。
```
