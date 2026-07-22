# LM Talk 文档索引

这里是 `docs/` 的中文入口。建议先阅读总览，再按需要进入协议、安全、部署、测试或发布文档。

## 总览与路线

| 文档 | 内容 |
| --- | --- |
| [DESIGN.md](overview/DESIGN.md) | 系统目标、整体架构、协议边界和当前实现状态。 |
| [MVP_PLAN.md](overview/MVP_PLAN.md) | MVP 能力轨道、阶段目标和长期增强方向。 |
| [PRODUCTION_READINESS.md](overview/PRODUCTION_READINESS.md) | 当前功能就绪看板和剩余打磨项。 |
| [TODO.md](overview/TODO.md) | 当前仍需继续设计或实现的事项。 |
| [DEV_WORKFLOW.md](overview/DEV_WORKFLOW.md) | 本地开发、验证、节点运行和常用脚本。 |

## 协议规格

| 文档 | 内容 |
| --- | --- |
| [IDENTITY_SPEC.md](protocol/IDENTITY_SPEC.md) | 身份种子、密钥派生、User ID 和提示词归一化。 |
| [BACKUP_SPEC.md](protocol/BACKUP_SPEC.md) | 身份备份包格式和恢复规则。 |
| [CONTACT_SPEC.md](protocol/CONTACT_SPEC.md) | 联系人名片、签名字段、信任等级和指纹展示。 |
| [FRIEND_SPEC.md](protocol/FRIEND_SPEC.md) | 好友请求/响应格式、签名和投递方式。 |
| [MESSAGE_SPEC.md](protocol/MESSAGE_SPEC.md) | 消息 envelope、加密、ACK 和回执边界。 |
| [GROUP_SPEC.md](protocol/GROUP_SPEC.md) | 群聊 fanout、Sender Key、群事件和权限边界。 |
| [PUBLIC_PEER_SPEC.md](protocol/PUBLIC_PEER_SPEC.md) | PublicPeer、Mailbox 消息和反滥用要求。 |
| [NETWORK_SPEC.md](protocol/NETWORK_SPEC.md) | 网络策略、控制面鉴权和跨设备部署方式。 |
| [STORAGE_SPEC.md](protocol/STORAGE_SPEC.md) | Web IndexedDB、本地加密字段和迁移要求。 |
| [DIAGNOSTICS_SPEC.md](protocol/DIAGNOSTICS_SPEC.md) | 诊断报告允许字段、禁止字段和分享流程。 |

## 安全与密码学

| 文档 | 内容 |
| --- | --- |
| [SECURITY_MODEL.md](security/SECURITY_MODEL.md) | 安全目标、非目标、威胁边界和已知限制。 |
| [CRYPTO_REVIEW_NOTES.md](security/CRYPTO_REVIEW_NOTES.md) | 密码学设计说明、生命周期、信任边界和 strict E2EE 控制消息例外。 |
| [SECURITY_AUDIT_SCOPE.md](security/SECURITY_AUDIT_SCOPE.md) | 内部安全自查范围和未来外部审计参考。 |
| [EXTERNAL_AUDIT_PACKET.md](security/EXTERNAL_AUDIT_PACKET.md) | 未来外部审计入口包。 |
| [AUDIT_REMEDIATION_TRACKER.md](security/AUDIT_REMEDIATION_TRACKER.md) | 审计发现和修复跟踪模板。 |
| [INCIDENT_RESPONSE_RUNBOOK.md](security/INCIDENT_RESPONSE_RUNBOOK.md) | 安全事件响应手册。 |
| [DEPENDENCY_RISK_REVIEW.md](security/DEPENDENCY_RISK_REVIEW.md) | 依赖风险复核和审计例外说明。 |

## 节点、部署与运维

| 文档 | 内容 |
| --- | --- |
| [LAN_QUICKSTART.md](deploy/LAN_QUICKSTART.md) | 一条脚本命令启动局域网 Web 页面和同步服务。 |
| [NODE_CONFIG.md](deploy/NODE_CONFIG.md) | `lm_node serve-control` 配置文件、字段和运维说明。 |
| [PUBLIC_FEDERATION_RUNBOOK.md](deploy/PUBLIC_FEDERATION_RUNBOOK.md) | 公网联邦节点运行说明。 |
| [PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md](deploy/PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md) | 可选公网部署报告模板。 |

## 测试与协议稳定

| 文档 | 内容 |
| --- | --- |
| [TEST_VECTOR_COVERAGE.md](testing/TEST_VECTOR_COVERAGE.md) | 测试向量覆盖状态。 |
| [PROTOCOL_STABILITY.md](testing/PROTOCOL_STABILITY.md) | 协议稳定性、冻结规则和兼容策略。 |

## 发布参考

当前目标以功能可用为主，发布/证据/签名/风险类文档是可选发布参考，不作为当前目标阻塞项。

| 文档 | 内容 |
| --- | --- |
| [RELEASE_CHECKLIST.md](release/RELEASE_CHECKLIST.md) | 发布检查步骤。 |
| [RELEASE_EVIDENCE.md](release/RELEASE_EVIDENCE.md) | 可选发布证据模板。 |
| [RELEASE_RISK_REGISTER.md](release/RELEASE_RISK_REGISTER.md) | 已知限制和后续增强项。 |
| [RELEASE_SIGNING.md](release/RELEASE_SIGNING.md) | 产物校验和可选平台签名说明。 |
| [RELEASE_SIGNOFF.md](release/RELEASE_SIGNOFF.md) | 可选发布签核模板。 |


## 分类边界与去重规则

为避免同一内容在多个文档里反复维护，后续新增或移动文档时按下面规则归类：

| 类别 | 放置位置 | 只记录什么 | 不重复记录什么 |
| --- | --- | --- | --- |
| 总览/路线 | `docs/overview/` | 项目目标、当前状态、待办优先级 | 具体 wire 字段、发布证据表 |
| 协议规格 | `docs/protocol/` | 稳定对象格式、字段、验证规则 | 运维步骤、UI 操作教程 |
| 安全 | `docs/security/` | 威胁模型、密码学说明、审计范围、事件响应 | 发布签核、部署执行记录 |
| 部署 | `docs/deploy/` | 节点配置、公网运行手册、部署报告模板 | 协议字段完整定义、发布决策 |
| 发布 | `docs/release/` | release check、证据索引、风险登记、签名/签核模板 | 公网部署细节和协议字段定义 |

模板类文档的边界：

- `docs/release/RELEASE_EVIDENCE.md`：记录一次 release candidate 的检查证据索引。
- `docs/release/RELEASE_SIGNOFF.md`：记录一次发布是否放行的人工签核结论。
- `docs/deploy/PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md`：记录一次公网或准公网部署现场报告。
- `docs/security/AUDIT_REMEDIATION_TRACKER.md`：记录安全审计发现和修复跟踪。

若某段内容同时适合多个文档，优先保留在最细分类文档中，其他位置只放 1 行链接。

## 测试向量

固定测试向量位于仓库根目录：

```text
test-vectors/
```

当前覆盖身份、备份、联系人名片、好友请求、消息加密、设备、PreKey、Ratchet、DHT、self-sync、文件、群聊和 per-device envelope。详细见 [TEST_VECTOR_COVERAGE.md](testing/TEST_VECTOR_COVERAGE.md)。

## 文档维护约定

- `docs/` 默认使用中文。
- 命令、字段名、协议前缀和文件名保持原文。
- `overview/DESIGN.md` 记录跨模块总设计，不放过细字段定义。
- `protocol/*_SPEC.md` 记录稳定协议和数据格式。
- `deploy/NODE_CONFIG.md` 记录节点部署和运维细节。
- `overview/TODO.md` 只记录当前仍需处理的功能事项。
- 发布、审计、风险和证据类文档应标明“可选生产增强”或“当前目标必需”。
