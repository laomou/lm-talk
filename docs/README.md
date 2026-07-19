# LM Talk 文档索引

本文档是 `docs/` 目录的中文入口。阅读建议：先看"总览与路线"，再按需要进入协议、安全、部署、发布或测试文档。

## 总览与路线

| 文档 | 内容 |
| --- | --- |
| [DESIGN.md](overview/DESIGN.md) | 系统目标、整体架构、协议边界和当前实现状态。 |
| [MVP_PLAN.md](overview/MVP_PLAN.md) | MVP 能力轨道、阶段目标和长期增强方向。 |
| [PRODUCTION_READINESS.md](overview/PRODUCTION_READINESS.md) | 当前功能就绪看板、已完成能力和剩余打磨项。 |
| [TODO.md](overview/TODO.md) | 遗留事项和需要继续设计的功能点。 |

## 核心协议规格

| 文档 | 内容 |
| --- | --- |
| [IDENTITY_SPEC.md](protocol/IDENTITY_SPEC.md) | 身份种子、密钥派生、User ID 和提示词归一化。 |
| [BACKUP_SPEC.md](protocol/BACKUP_SPEC.md) | 身份备份包格式、Argon2id 标准路径和 WASM 兼容路径。 |
| [CONTACT_SPEC.md](protocol/CONTACT_SPEC.md) | 联系人名片、签名字段、信任等级和指纹展示。 |
| [FRIEND_SPEC.md](protocol/FRIEND_SPEC.md) | 好友请求/响应格式、签名和投递方式。 |
| [MESSAGE_SPEC.md](protocol/MESSAGE_SPEC.md) | Direct envelope、crypto id、AAD、ACK 和回执边界。 |
| [GROUP_SPEC.md](protocol/GROUP_SPEC.md) | 群聊 fanout、Sender Key、群事件和权限边界。 |
| [PUBLIC_PEER_SPEC.md](protocol/PUBLIC_PEER_SPEC.md) | Public Peer announce、Mailbox 消息和反滥用要求。 |
| [NETWORK_SPEC.md](protocol/NETWORK_SPEC.md) | 网络策略、控制面鉴权和跨设备部署方式。 |
| [STORAGE_SPEC.md](protocol/STORAGE_SPEC.md) | Web IndexedDB、本地加密字段和迁移要求。 |
| [DIAGNOSTICS_SPEC.md](protocol/DIAGNOSTICS_SPEC.md) | 诊断报告允许字段、禁止字段和用户分享流程。 |

## 安全、密码学与事件响应

| 文档 | 内容 |
| --- | --- |
| [SECURITY_MODEL.md](security/SECURITY_MODEL.md) | 安全目标、非目标、威胁边界和已知限制。 |
| [SECURITY_AUDIT_SCOPE.md](security/SECURITY_AUDIT_SCOPE.md) | 安全审查范围、重点路径和交付要求。 |
| [CRYPTO_REVIEW_NOTES.md](security/CRYPTO_REVIEW_NOTES.md) | 密码学设计说明、生命周期、信任边界和 strict E2EE 控制消息例外。 |
| [EXTERNAL_AUDIT_PACKET.md](security/EXTERNAL_AUDIT_PACKET.md) | 外部审计入口包：代码地图、命令、审计问题和交付物。 |
| [AUDIT_REMEDIATION_TRACKER.md](security/AUDIT_REMEDIATION_TRACKER.md) | 审计发现、修复状态、验证证据和发布决策跟踪。 |
| [INCIDENT_RESPONSE_RUNBOOK.md](security/INCIDENT_RESPONSE_RUNBOOK.md) | 安全事件分级、处置步骤、沟通和恢复流程。 |
| [DEPENDENCY_RISK_REVIEW.md](security/DEPENDENCY_RISK_REVIEW.md) | 依赖风险、审计例外和升级复核要求。 |

## 节点、部署与运维

| 文档 | 内容 |
| --- | --- |
| [NODE_CONFIG.md](deploy/NODE_CONFIG.md) | `lm_node serve-control` 配置文件、字段、备份恢复和运维说明。 |
| [PUBLIC_FEDERATION_RUNBOOK.md](deploy/PUBLIC_FEDERATION_RUNBOOK.md) | 公网联邦节点运行手册、拓扑、TLS、监控和验证流程。 |
| [PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md](deploy/PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md) | 公网部署报告模板，用于记录拓扑、配置、测试和 go/no-go 结论。 |

## 测试、fuzz 与协议稳定

| 文档 | 内容 |
| --- | --- |
| [TEST_VECTOR_COVERAGE.md](testing/TEST_VECTOR_COVERAGE.md) | 测试向量覆盖状态、已覆盖协议对象和缺口。 |
| [PROTOCOL_STABILITY.md](testing/PROTOCOL_STABILITY.md) | 协议稳定性、冻结规则、兼容策略和升级要求。 |
| [FUZZING.md](testing/FUZZING.md) | 模糊测试目标、命令、报告和语料/崩溃处理约定。 |

## 发布、证据与签核

| 文档 | 内容 |
| --- | --- |
| [RELEASE_CHECKLIST.md](release/RELEASE_CHECKLIST.md) | 发布检查清单、自动 gate、原生产物和 evidence 收集入口。 |
| [RELEASE_EVIDENCE.md](release/RELEASE_EVIDENCE.md) | 发布证据索引模板，用于归档测试、部署、审计和产物证明。 |
| [RELEASE_RISK_REGISTER.md](release/RELEASE_RISK_REGISTER.md) | 发布风险登记、风险状态、接受规则和机器 gate。 |
| [RELEASE_SIGNING.md](release/RELEASE_SIGNING.md) | 原生节点发布签名、公证和签名证据报告说明。 |
| [RELEASE_SIGNOFF.md](release/RELEASE_SIGNOFF.md) | 发布签核模板，记录最终 go/no-go、审核人和证据链接。 |

## 测试向量位置

固定测试向量位于仓库根目录：

```text
test-vectors/
```

当前覆盖身份、备份、联系人名片、好友请求、消息加密、设备、PreKey、Ratchet、DHT、self-sync、文件、群聊和 per-device envelope 等对象。详细覆盖状态见 [TEST_VECTOR_COVERAGE.md](testing/TEST_VECTOR_COVERAGE.md)。

## Docker / federation 测试入口

三节点 federation 模板与测试脚本(smoke/chaos/load/run-all)同处一个目录：

```text
deploy/lm-node-federation/
```

常用命令：

```bash
# 使用 deploy/lm-node-federation/docker-compose.yml 启动并运行完整 smoke/chaos/load
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json \
  deploy/lm-node-federation/run-all.sh
```

## 文档维护约定

- `docs/` 内默认使用中文；需要英文时可在同文档中附英文摘要，或单独维护英文版。
- `README.md` / 仓库根 README 作为默认英文入口时，中文入口使用 `README_zh.md`。
- `DESIGN.md` 记录跨模块总设计和实现状态，不放过细字段定义。
- `*_SPEC.md` 记录稳定协议和数据格式，尽量短、准、可实现。
- `NODE_CONFIG.md` 只记录节点部署和运维细节。
- `TODO.md` 只记录尚未完成或仍需设计的事项；已完成内容应同步到 `DESIGN.md`、规格或对应专题文档。
- 发布、审计、风险和证据类文档应保持互相链接，避免结论散落。
