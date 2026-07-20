# 发布风险登记

本文用于记录 LM Talk 的已知限制和后续增强项。当前功能目标不要求补齐 owner、发布决策或证据链接；风险登记仅作为维护参考。

## 状态值

- `Open`：仍需关注。
- `Mitigated`：已有缓解措施。
- `Accepted`：明确接受该限制。
- `Closed`：不再适用。

## 当前已知限制

| ID | 风险 / 限制 | 严重性 | 状态 | 当前缓解 |
| --- | --- | --- | --- | --- |
| RISK-001 | LM Talk 保护内容，但不提供匿名性或抗流量分析。User ID、时间、DHT key、记录类型和消息大小可能可观察。 | Medium | Open | 安全模型与密码学说明中明确非匿名目标；产品文案不应宣称匿名。 |
| RISK-002 | 公共节点可观察元数据并影响可用性，但不能解密 E2EE 内容。 | Medium | Open | 多节点 federation、Mailbox/DHT、strict E2EE 和本地优先设计降低单点影响。 |
| RISK-003 | DirectEnvelope 和 per-device fallback 仍作为兼容路径存在。 | Medium | Mitigated | 新身份默认 strict E2EE；内容路径要求 verified contact 和 sealed slot；修复控制消息例外已记录。 |
| RISK-004 | 浏览器或本机运行时被完全控制时，实时明文和密钥可能暴露。 | High | Open | IndexedDB 加密、身份删除、重加密和备份提示；不承诺防御完全控制运行时。 |
| RISK-005 | 原生 macOS/Windows 产物未必具备平台级签名/公证。 | Medium | Open | 发布包提供 SHA256 校验；平台签名作为可选生产发行增强。 |
| RISK-006 | 长时间 fuzz、长期公网 load/chaos 不作为当前目标阻塞项。 | Medium | Open | 提供 fuzz smoke、Docker federation smoke/chaos/load 和本地验证脚本。 |
| RISK-007 | 第三方安全审计不作为当前目标阻塞项。 | Medium | Open | 提供审计范围、密码学说明和测试向量，便于未来审查。 |
| RISK-008 | 依赖 advisory 例外可能随功能变化变为可达。 | Medium | Open | `scripts/check-audit.sh` 与依赖风险复核文档记录例外理由。 |
| RISK-009 | 公网 federation 可用性取决于运营者部署质量。 | Medium | Open | 提供 Docker federation 模板、节点配置文档和运行手册。 |

## 当前功能目标下的使用方式

- 本文件不再作为当前目标的阻塞门禁。
- 可在发布说明、issue 或回归测试计划中引用相关风险。
- 如果未来要做公开生产发行，可重新启用风险 owner、发布决策和证据链接流程。

## 可选机器检查

如需生成风险检查报告，可运行：

```bash
RISK_REGISTER_GATE_MODE=report ./scripts/release-risk-gate.sh
```

该报告仅供参考，不作为当前功能目标完成条件。
