# 功能就绪看板

本文档用于跟踪 LM Talk 作为**去中心化、端到端加密即时通讯应用**的功能完成度。当前目标不再把第三方审计、长时间 fuzz、公网长期压测、真实公网部署报告、macOS notarization / Windows code signing、风险登记签核作为完成阻塞项；这些仍可作为后续发布工程或商业分发工作单独维护。

## 总体状态

| 领域 | 状态 | 说明 |
| --- | --- | --- |
| Web MVP / Demo 可用性 | 已完成，持续打磨 | 注册、登录、联系人、聊天、文件、群聊、同步、备份、诊断均可用。 |
| 端到端内容加密 | 已完成 | 一对一消息、文件包、群 Sender Key、分设备 sealed slot 均已实现。 |
| 去中心化发现与投递 | 已完成，继续增强体验 | ContactCard / PreKey / MailboxHint / PublicPeer DHT 与 Mailbox 离线投递可用。 |
| 多设备 E2EE | 已完成，继续增强互操作 | 设备证书、设备撤销、自同步、sealed slot、设备证书 fanout 与 ACK 均已实现。 |
| Strict E2EE 默认策略 | 已完成 | 新身份默认启用 verified-contact + sealed-slot 收发策略；核心风险 fail-closed。 |
| 本地数据保护 | 已完成，继续增强平台覆盖 | Web IndexedDB 应用层加密、重加密、身份级删除、完整数据备份可用；Native state_db 支持 SQLite/SQLCipher/external 模式。 |
| Native node / Docker 部署模板 | 已完成，测试中 | 公共节点与三节点 federation 模板存在；Docker smoke 脚本已迁移到 `tests/deploy/lm-node-federation/`。 |
| 证据/诊断导出 | 已完成，持续补充 | 诊断报告、strict E2EE readiness 报告、release evidence helpers 已具备。 |

## 功能完成度估计

| 目标 | 当前估计 | 备注 |
| --- | --- | --- |
| 可用 Demo / MVP | 约 98% | 核心用户路径可跑通，主要剩余是体验和边界打磨。 |
| 功能目标整体 | 约 98% | 已具备去中心化发现/投递与 E2EE 主体能力。 |

## 已完成的核心能力

### 身份与本地数据

- 身份创建、恢复、重加密和本机身份列表。
- 新身份默认生成本设备证书与设备私钥备份。
- 旧身份/导入身份登录或同步前会自动补齐本设备 sealed slot 证书，并向好友 fanout 设备证书更新。
- Web IndexedDB 敏感字段应用层加密。
- 本地身份删除会清理对应身份的数据分表。
- 完整数据备份导出、导入合并、导入覆盖和自己的 Mailbox 备份。

### 联系人与信任

- ContactCard 签名与验证。
- 联系人指纹展示、复制、二维码、扫码/文本核验。
- 本地信任状态在 ContactCard 合并中保留。
- 设备证书合并、设备撤销、撤销优先策略。
- ContactCard DHT 发布、发现、后台刷新。

### 消息与文件

- 一对一 E2EE 消息发送/接收。
- X3DH / PreKey 与 Double Ratchet 路径。
- 文件加密包生成、发送、接收、解密和下载标记。
- Strict E2EE 下文件发送、Mailbox 文件接收、手动文件解密均执行联系人信任策略。
- 回执、已读回执、Mailbox delivery id 和 self-sync receipt state 合并。

### 群聊

- 群聊创建、邀请、接受、退出。
- 群 Sender Key 加密与群消息 fanout。
- 群事件：改名、加人、移除、管理员变更等。
- 群成员 strict E2EE 风险检测、修复向导、发送前提示和严格阻断。
- strict 模式下阻止风险群创建、风险群邀请接受、风险群消息、风险群事件 fanout。

### Strict E2EE 默认与修复闭环

- 新身份默认启用：
  - `requireVerifiedContactsForSend`
  - `requireVerifiedContactsForReceive`
  - `requireSealedPerDeviceSlotsForSend`
  - `requireSealedPerDeviceSlotsForReceive`
- 核心风险 fail-closed：未核验指纹、撤销设备、缺少 sealed slot 支持。
- 非核心新鲜度风险可见但不阻断：ContactCard DHT 未刷新、设备更新 ACK 待确认。
- strict E2EE readiness 报告可复制/下载，支持完整报告和脱敏摘要。
- 通讯录可按“严格 E2EE 阻塞”筛选联系人和群聊。

### 去中心化网络与节点

- DHT record 类型：PublicPeer、PreKey、MailboxHint、ContactCard。
- Mailbox push/take/ack/status。
- Native node 控制面鉴权、CORS、速率限制、大小限制。
- Snapshot sync、DHT replication、routing refresh、peer health/quarantine 指标。
- 三节点 federation Docker 模板和 smoke/chaos/load 测试脚本。

## 仍可继续打磨的功能项

这些不是当前目标的完成阻塞项，但适合继续迭代：

1. **Docker federation 测试稳定化**
   - 让 `tests/deploy/lm-node-federation/run-all.sh` 在无 compose plugin 或本地 direct docker 模式下更稳定。
   - 统一 `/mailbox/status` 与 `/mailbox/take` 的测试断言字段。

2. **Strict E2EE 体验细化**
   - 将“核心阻断风险”和“非阻断新鲜度提醒”在更多页面中用不同颜色/标签区分。
   - 给每个风险项提供更短的修复路径。

3. **群聊成员管理体验**
   - 在添加/移除成员前显示成员 strict E2EE 状态。
   - 群成员列表支持按风险排序。

4. **移动端/窄屏体验**
   - 聊天页、通讯录、设置页在小屏下继续优化布局。

5. **运维易用性**
   - Docker compose plugin 缺失时给出更明确提示。
   - 一键启动 federation 测试时自动生成 secrets、自动清理本地测试产物。

## 不作为当前目标阻塞项的事项

以下事项可以后续另立发布/合规/商业分发目标，不再阻塞“去中心化 E2EE IM 功能目标”判断：

- 第三方安全审计报告。
- 长时间 fuzz campaign 产物与 crash triage。
- 长时间公网 federation chaos/load 证据。
- 真实公网部署报告。
- macOS notarization / Windows code signing。
- 风险登记 owner / release decision / evidence 的完整签核。
