# 发布签核

使用本文件作为生产发布候选的最终 Go/No-Go 决策。它与 `docs/RELEASE_EVIDENCE.md`、`docs/PRODUCTION_READINESS.md`、`docs/PROTOCOL_STABILITY.md` 和 `docs/SECURITY_AUDIT_SCOPE.md` 共同补充。

## 发布候选

- 版本/标签：
- 提交 SHA：
- 发布分支：
- UTC 日期/时间：
- 发布负责人：
- 安全审核人：
- 运营审核人：

## 所需证据链接

| 证据 | 链接 / 产物 | 审核人 | 状态 |
| --- | --- | --- | --- |
| 已完成的 `docs/RELEASE_EVIDENCE.md` 副本 |  |  |  |
| `./scripts/release-check.sh full` 输出 |  |  |  |
| 依赖审计 / 依赖复核 |  |  |  |
| SQLCipher smoke 工作流产物 |  |  |  |
| SQLCipher 发布二进制 smoke 产物 |  |  |  |
| 联邦 `run-all.sh` 报告 |  |  |  |
| 联邦 chaos/load 报告 |  |  |  |
| Fuzz smoke 报告 |  |  |  |
| 长时 fuzz 活动报告/语料/崩溃分类 |  |  |  |
| 外部安全审计报告 |  |  |  |
| 审计修复提交 |  |  |  |
| 公网部署拓扑和配置 |  |  |  |
| macOS 公证证据 |  |  |  |
| Windows 代码签名证据 |  |  |  |
| SECURITY.md 审查 |  |  |  |
| 已完成 `docs/RELEASE_RISK_REGISTER.md` 审查 |  |  |  |

## 协议冻结签核

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 稳定对象测试向量通过 |  |  |
| DHT 记录类型/命名空间冻结 |  |  |
| Mailbox 类型映射冻结 |  |  |
| ContactCard/DeviceCert 合并策略测试 |  |  |
| PreKey 轮换/消费互操作性测试 |  |  |
| 错误文本/代码依赖审查 |  |  |
| 废弃/回退策略已接受 |  |  |

## 安全签核

| 项目 | 决策 / 备注 | 状态 |
| --- | --- | --- |
| Critical 发现已修复 |  |  |
| High 发现已修复或明确接受 |  |  |
| Medium/Low 发现已分类 |  |  |
| 风险登记已审查：无未认领 Critical/High 风险 |  |  |
| 已接受已知元数据泄露 |  |  |
| 严格 E2EE 降级/回退策略已接受 |  |  |
| SQLCipher 部署证据已接受 |  |  |
| 令牌/CORS/部署指南已接受 |  |  |

## 运营签核

| 项目 | 证据 / 备注 | 状态 |
| --- | --- | --- |
| 公共引导/联邦拓扑已验证 |  |  |
| 备份/恢复演练已完成 |  |  |
| 监控端点已归档 |  |  |
| 事件/联系方式流程当前有效 |  |  |
| 回滚方案已记录 |  |  |
| 产物校验和已验证 |  |  |

## 本次发布已知限制

列出本次发布明确接受的限制。除非有负责人和缓解措施，否则不要将阻塞项列为已接受限制。

- 

## Go / No-Go 决策

- 决策：`GO` / `NO-GO`
- 需要后续问题：
- 批准人：
- 批准日期/时间 UTC：

如果缺少以下任何一项，生产发布应为 **NO-GO**：

1. 外部审计报告和修复评审。
2. 长时 fuzz 活动报告和崩溃分类。
3. 来自真实拓扑的联邦 chaos/load 证据。
4. 当使用 state_db 时，SQLCipher 发布产物 smoke 证据。
5. 生产桌面/原生分发的 macOS 公证 / Windows 签名证据。
6. 已完成的发布证据索引。
7. 已完成的风险登记，且无未认领的 open Critical/High 风险。
