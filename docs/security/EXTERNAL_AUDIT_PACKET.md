# 外部安全审计交付包

本交付包是面向审计方的入口，用于审查 LM Talk 作为去中心化端到端加密即时消息系统。它本身不声称生产就绪；它索引了外部审计必须验证的确切领域、证据和未决阻塞项。

## 审计目标

- 产品目标：具有 Web/WASM 客户端和原生公共/联邦节点的去中心化 E2EE 即时消息。
- 主要安全目标：消息/文件/群组内容机密性、签名身份/联系人名片/设备、严格 E2EE 降级控制、加密本地持久化和恶意节点弹性。
- 当前分发目标：基于标签的 Linux、macOS、Windows `lm_node` 原生产物，以及 Web 静态构建证据。

## 必填提交和发布标识

在发送审计包前填写：

| 字段 | 值 |
| --- | --- |
| 审计提交 SHA | `TODO` |
| 发布候选标签 | `TODO` |
| 证据目录或 CI 运行 | `TODO` |
| 审计员 / 审计公司 | `TODO` |
| 审计启动日期 | `TODO` |
| 审计负责人 | `TODO` |

## 从这里开始

1. `docs/SECURITY_AUDIT_SCOPE.md` — 证明范围和所需审计交付物的权威索引。
2. `docs/SECURITY_MODEL.md` — 简明安全目标、非目标和产品边界。
3. `docs/CRYPTO_REVIEW_NOTES.md` — 协议/密码学设计说明和已知元数据限制。
4. `docs/PROTOCOL_STABILITY.md` — 协议冻结状态和兼容性规则。
5. `docs/TEST_VECTOR_COVERAGE.md` — 已签名/加密线对象的规范向量覆盖。
6. `docs/RELEASE_RISK_REGISTER.md` — 需要缓解、接受或标记为否决的残余风险。
7. `docs/AUDIT_REMEDIATION_TRACKER.md` — 审计过程中更新的发现/修复跟踪。
8. `docs/RELEASE_EVIDENCE.md` 和 `docs/RELEASE_SIGNOFF.md` — 审计发布的证据/签核模板。

## 代码审查地图

| 领域 | 路径 | 审查重点 |
| --- | --- | --- |
| 核心密码/协议 | `crates/lm_core/src`, `test-vectors/`, `crates/lm_core/tests` | 身份、ContactCard、设备证书/撤销、X3DH、Double Ratchet、群组发送密钥、文件包、回执、过期/版本/大小检查。 |
| WASM 桥接 | `crates/lm_wasm/src` | JS/WASM 边界验证、错误映射、密码学辅助暴露、每设备槽封装/打开。 |
| Web 客户端 | `apps/web/src`, `apps/web/tests` | 严格 E2EE 预检、指纹/设备 UX、IndexedDB 加密、备份/导入、自同步、封闭槽降级阻断、本地删除/重新加密。 |
| 原生节点 | `crates/lm_node/src` | 控制面解析/认证/限流、Mailbox/PreKey/DHT 行为、对等隔离、SQLCipher 提供器、指标、持久化恢复。 |
| 联邦部署 | `deploy/lm-node-public`, `deploy/lm-node-federation` | TLS/Caddy 模板、密钥挂载、CORS 源、暴露端口、加密卷假设。 |
| 发布供应链 | `.github/workflows`, `scripts/package-node-release.py`, `scripts/verify-node-release.sh`, `scripts/preprod-evidence.sh` | 跨平台构建、校验和、SQLCipher 产物证明、证据采集、依赖审计门禁。 |

## 测试和证据命令

审计员应运行或审查审核提交/标签的归档输出：

```bash
./scripts/release-check.sh full
./scripts/audit.sh
./scripts/risk-register-gate.sh
FUZZ_SMOKE_REPORT=fuzz-smoke-report.json ./scripts/fuzz-smoke.sh
FUZZ_CAMPAIGN_DURATION=3600 ./scripts/fuzz-campaign.sh
./scripts/sqlcipher-smoke.sh
./scripts/sqlcipher-deploy-smoke.sh
RUN_RELEASE_ASSET_VERIFY=1 RELEASE_TAG_VERIFY=<tag> RELEASE_VERSION=<tag> ./scripts/preprod-evidence.sh
```

对于联邦证据：

```bash
tests/deploy/lm-node-federation/run-all.sh
tests/deploy/lm-node-federation/chaos-smoke.sh
MESSAGE_COUNT=100 tests/deploy/lm-node-federation/load-smoke.sh
```

生产就绪的长期活动应将 fuzz、混沌和负载持续时间提高到超出 smoke 默认值，并归档语料/崩溃目录和 JSON 报告。

## 高优先级审计问题

- 是否任何不可信节点、DHT 对等节点、Mailbox 运营商或中继能观察或修改消息明文？
- 严格封闭每设备槽是否在启用时强制执行且不会静默回退？
- ContactCards、设备证书、撤销、PreKeys、回执和 DHT 记录是否绑定到预期的身份/密钥/版本/过期？
- 是否重放、重排、重复、延迟、超大、畸形和过期对象被拒绝或安全处理？
- 自同步是否保留信任/设备/撤销状态，而不复活陈旧或撤销设备？
- 原生节点限额/限流是否防止实际未授权或令牌认证的资源耗尽？
- SQLCipher 模式是否 fail-closed，并为确切发布产物报告可靠的加密状态指标？
- 发布产物和依赖审计例外是否避免可达的供应链风险？

## 已知发布阻塞项待验证

以下项在没有外部证据前预计仍为否决：

- 已完成的第三方审计报告和修复验证。
- 长时 fuzz 活动产物和分类笔记。
- 真实拓扑的长时公共联邦混沌/负载报告。
- macOS 公证和 Windows 代码签名的生产信任证据。
- 已完成的 `docs/RELEASE_SIGNOFF.md` 和 `docs/RELEASE_RISK_REGISTER.md` 中的已解决/已接受条目。

## 审计输出要求

审计员发现应包括：

1. 严重性、受影响组件/路径和利用叙述。
2. 重现步骤、载荷或测试向量（如适用）。
3. 建议修复和验证方法。
4. 审查提交 SHA 和发布标签。
5. 对任何已接受限制的显式残余风险声明。

在 `docs/AUDIT_REMEDIATION_TRACKER.md` 中跟踪发现，并在生产签核前链接修复提交、测试和发布证据。
