# 发布风险登记

使用本登记跟踪在实现、测试和审计修复后仍然存在的残余风险。生产发布必须明确接受或解决每个非低残余风险。

## 状态值

- `Open`：风险存在，需要缓解或接受。
- `Mitigated`：已实施缓解并关联证据。
- `Accepted`：发布负责人和安全审核人接受该残余风险。
- `Rejected`：风险不被接受；在修复前发布为否决。
- `Closed`：风险不再适用。

## 风险登记

| ID | 风险 | 严重性 | 状态 | 缓解 / 证据 | 所需证据 | 证据链接 | 负责人 | 发布决策 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| RISK-001 | LM Talk 保护内容，但不提供匿名性或流量分析抗性；用户 ID、时间、DHT 密钥、记录类型和消息大小可能可观察。 | Medium | Open | 已记录于 `docs/CRYPTO_REVIEW_NOTES.md`；公众消息必须避免匿名性声明。 | 发布说明措辞和产品/安全文档确认不做匿名性声明。 | `docs/CRYPTO_REVIEW_NOTES.md`; TODO 发布说明。 | TODO(product/security owner) | TODO(no-go/accepted with disclosure) |
| RISK-002 | 公共节点运营商可以观察元数据，并可能降低可用性，即使他们无法解密 E2EE 内容。 | Medium | Open | 联邦、多节点故障转移、严格 E2EE 和公网部署运行手册；需要真实公开拓扑证据。 | 公共联邦部署报告、拓扑、可用性/故障转移说明和运营商披露。 | `docs/PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md`; TODO 公共报告。 | TODO(ops owner) | TODO(no-go/accepted with disclosure) |
| RISK-003 | Legacy DirectEnvelope 和占位符 per-device slots 仍为兼容路径，可能比严格密封槽弱。 | Medium | Open | 严格 E2EE 策略阻止回退；UI 警告和预检存在；生产默认前需要废弃计划。 | 严格 E2EE 证据、降级测试、废弃/兼容性决策和发布说明措辞。 | `docs/CRYPTO_REVIEW_NOTES.md`; TODO 严格模式证据。 | TODO(security owner) | TODO(no-go/mitigated/accepted) |
| RISK-004 | 浏览器/本地设备妥协可能在用户登录时暴露实时解密数据或密钥。 | High | Open | 已有 IndexedDB 加密和本地删除机制；无法防御完全妥协的运行时。需要明确用户指南和审计审核。 | 外部审计声明、用户指南，以及发布负责人和安全审核人的残余风险接受决策。 | `docs/SECURITY_MODEL.md`; TODO 审计报告。 | TODO(security reviewer) | TODO(no-go/accepted) |
| RISK-005 | macOS 公证和 Windows 代码签名尚未为生产信任原生分发实现。 | High | Open | 发布检查清单将其标记为生产分发前的必需项。 | macOS codesign/notary/staple 验证、Windows 签名验证和签名/公证证据报告。 | TODO `signing-evidence.json`; TODO notarization log; TODO signtool log. | TODO(build/release owner) | TODO(no-go until signed) |
| RISK-006 | 长时 fuzz、混沌和负载证据已有脚本/模板，但不一定完成生产时长运行。 | High | Open | 已有 `scripts/fuzz-campaign.sh`、联邦运行手册和证据模板；发布必须归档真实报告。 | 长时 fuzz 活动报告及语料/崩溃分类，以及长时联邦混沌/负载报告。 | TODO fuzz report; TODO federation report. | TODO(test/release owner) | TODO(no-go until evidence archived) |
| RISK-007 | 外部安全审计有范围/跟踪，但仓库证据中没有完成的第三方报告。 | High | Open | `docs/SECURITY_AUDIT_SCOPE.md` 和 `docs/AUDIT_REMEDIATION_TRACKER.md`；在没有审计报告/修复前发布为否决。 | 第三方审计报告、审查提交/标签、修复跟踪和接受残余风险声明。 | `docs/EXTERNAL_AUDIT_PACKET.md`; TODO 审计报告。 | TODO(security owner) | TODO(no-go until audit complete) |
| RISK-008 | SQLCipher 安全性依赖于构建和部署正确的 SQLCipher 产物及强密码处理。 | Medium | Open | SQLCipher 功能、部署 smoke、发布 smoke 产物和证据检查清单；发布必须归档精确产物证明。 | SQLCipher 产物验证、部署 smoke JSON、证明已加密 state DB 的指标和密码处理审查。 | TODO `sqlcipher-release-smoke-report.json`; TODO release asset verifier. | TODO(ops/release owner) | TODO(no-go/mitigated) |
| RISK-009 | 依赖 advisory 异常可能在功能更改或上游依赖行为改变时变为可达。 | Medium | Open | `scripts/check-audit.sh` 记录了窄范围的忽略；依赖升级和功能更改时应重新评估。 | 当前依赖审计输出、依赖风险复核和针对发布的可接受异常文档。 | `docs/DEPENDENCY_RISK_REVIEW.md`; TODO 审计日志。 | TODO(dependency owner) | TODO(no-go/accepted with review) |
| RISK-010 | 公共联邦可用性取决于运营商部署质量、TLS/CORS 正确性、令牌卫生和备份操作。 | Medium | Open | 已有部署模板和运行手册；生产证据要求真实公共部署报告。 | 公共部署报告证明 TLS/CORS/令牌/备份操作和持续联邦检查。 | `docs/PUBLIC_FEDERATION_RUNBOOK.md`; TODO 公共部署报告。 | TODO(ops owner) | TODO(no-go/accepted with ops signoff) |

## 接受规则

- 生产发布不能接受 Critical 风险。
- High 风险需发布负责人和安全审核人书面接受，并附缓解和用户/运营沟通。
- Medium 风险需有文档化缓解或明确的发布说明披露。
- 已接受风险必须复制到发布候选的 `docs/RELEASE_SIGNOFF.md`。

## 机器门禁

在发布签核前运行生产风险门禁：

```bash
./scripts/release-risk-gate.sh
```

Strict 模式在任何 Medium/High/Critical 风险为 `Open` 或 `Rejected`、缺少负责人或缺少发布决策时退出非零。若要打印相同发现而不使更大证据收集任务失败：

```bash
RISK_REGISTER_GATE_MODE=report ./scripts/release-risk-gate.sh
```

生产发布不得绕过此门禁；应按照以上接受规则解决、缓解或明确接受每个非低残余风险。

## 审查清单

发布签核前：

- [ ] 每个 `Open` 的 High/Medium 风险都有负责人。
- [ ] 每个已接受风险都有证据和发布说明措辞。
- [ ] 每个已缓解风险链接修复提交/测试/产物。
- [ ] 不存在仍为 open 或已接受的 Critical 风险。
- [ ] 本登记已链接到 `docs/RELEASE_EVIDENCE.md`。
