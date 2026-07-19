# 审计修复跟踪

在每次内部或外部安全审计后使用本跟踪表。每个发现都必须在 LM Talk 被声明为生产就绪前包含负责人、状态、修复/缓解证据和发布决策。

## 状态值

- `Open`：发现已受理跟踪，尚未合并修复。
- `In progress`：修复或缓解正在实施中。
- `Fixed`：修复已合并并已验证。
- `Accepted risk`：该发布明确接受的残余风险，需附带缓解/理由。
- `Duplicate`：已被另一个发现覆盖。
- `Won't fix`：不适用或拒绝修复，需说明理由。

## 严重性值

- `Critical`：可利用导致 E2EE 内容、身份私钥或发布签名密钥泄露，或远程代码执行。
- `High`：严重信任绕过、设备撤销、严格 E2EE、DHT 污染保护、认证或持久性机密性绕过。
- `Medium`：有意义的安全降级，需具备前提条件或有限影响范围。
- `Low`：增强硬化、诊断泄露、或深度防御问题。
- `Info`：非安全或文档/流程类发现。

## 发现

| ID | 严重性 | 组件 | 摘要 | 状态 | 负责人 | 修复提交 | 验证证据 | 发布决策 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUDIT-0001 | TBD | TBD | 占位符：替换为首个审计发现。 | Open |  |  |  |  |

## 每个发现所需字段

每个发现都应在表格中或链接的问题中包含：

- 受影响组件：`lm_core`、`lm_wasm`、Web、`lm_node`、部署、CI/发布、docs。
- 威胁模型 / 利用叙述。
- 重现步骤或概念验证（在安全存储允许的前提下）。
- 期望的安全属性。
- 实际观察到的行为。
- 修复计划和负责人。
- 修复提交或已接受风险理由。
- 验证命令、测试或产物。
- 审核人签核。

## 发布门禁

如果任何发现满足以下条件，则生产发布为 **NO-GO**：

- `Critical` 且未标记为 `Fixed`。
- `High` 且未标记为 `Fixed`，或者未被发布负责人和安全审核人明确标记为 `Accepted risk`。
- 标记为 `Fixed` 后缺少验证证据。

在发布签核前，将最终发现摘要复制到 `docs/RELEASE_SIGNOFF.md` 并在 `docs/RELEASE_EVIDENCE.md` 中链接审计报告。

## 建议的问题标签

- `audit`
- `security`
- `severity:critical`
- `severity:high`
- `severity:medium`
- `component:core`
- `component:wasm`
- `component:web`
- `component:node`
- `component:deploy`
- `release-blocker`
