# 发布证据索引

使用本文件作为每个发布候选的证据模板。将其复制到发布专用文件或 issue 中，填写产物链接，并将完成后的副本与发布说明一起保存。

## 发布候选

- 版本/标签：
- 提交 SHA：
- UTC 日期/时间：
- 负责人/审核人：

## 必要自动门禁

| 门禁 | 所需证据 | 产物/链接 | 状态 |
| --- | --- | --- | --- |
| 快速发布检查 | `./scripts/release-check.sh quick` 或 CI `release-check` 日志 |  |  |
| 完整发布检查 | `./scripts/release-check.sh full` 输出 |  |  |
| 依赖审计 | `./scripts/check-audit.sh` / CI `dependency-audit` 日志 |  |  |
| 依赖风险复核 | 已审查 `docs/DEPENDENCY_RISK_REVIEW.md`；活动审计例外已说明 |  |  |
| SQLCipher smoke | `./scripts/check-sqlcipher.sh` 或 SQLCipher Smoke 工作流产物 |  |  |
| SQLCipher 发布二进制 smoke | 发布工作流中的 `lm_node-linux-x86_64-sqlcipher-smoke` 产物 |  |  |
| 联邦验证 | `tests/deploy/lm-node-federation/run-all.sh` 生成的 `federation-report.json` 或工作流产物 |  |  |
| 测试向量覆盖 | `cargo test -p lm_core --test test_vectors` 并审查 `docs/TEST_VECTOR_COVERAGE.md` 中的高优先级缺口 |  |  |
| Fuzz smoke | `FUZZ_SMOKE_REPORT=fuzz-smoke-report.json ./scripts/fuzz-smoke.sh` 输出/报告或 `./scripts/release-check.sh fuzz-smoke` 日志 |  |  |
| 长时 fuzz 活动 | `FUZZ_CAMPAIGN_DURATION=<seconds> FUZZ_CAMPAIGN_REPORT=fuzz-campaign-report.json ./scripts/fuzz-campaign.sh` 报告、日志、语料、产物和分类笔记 |  |  |

## 原生节点发布产物

| 产物 | 预期证据 | SHA256 / 链接 | 状态 |
| --- | --- | --- | --- |
| `lm_node-linux-x86_64.tar.gz` | `RELEASE_INFO.txt`、`.sha256` |  |  |
| `lm_node-linux-x86_64-sqlcipher.tar.gz` | 包含 `sqlcipher_enabled=true` 的 `RELEASE_INFO.txt`、`.sha256`、SQLCipher smoke 产物 |  |  |
| `lm_node-macos-x86_64.tar.gz` | `RELEASE_INFO.txt`、`.sha256` |  |  |
| `lm_node-macos-arm64.tar.gz` | `RELEASE_INFO.txt`、`.sha256` |  |  |
| `lm_node-windows-x86_64.zip` | `RELEASE_INFO.txt`、`.sha256` |  |  |
| `SHA256SUMS.txt` | 合并校验和文件 |  |  |

## 持久化 / 加密证据

| 模式 | 所需证明 | 产物/链接 | 状态 |
| --- | --- | --- | --- |
| SQLCipher `state_db` | `/control/stats` 显示 `state_db.encryption_mode=sqlcipher` 和 `state_db.encrypted=true` |  |  |
| SQLCipher 指标 | `/control/metrics` 包含 `lm_node_state_db_encrypted 1` 和 `lm_node_state_db_encryption_mode{mode="sqlcipher"} 1` |  |  |
| 错误密码短语 fail-closed | `sqlcipher-deploy-smoke.sh` 错误密码检查 |  |  |
| 加密 `state_file`（如使用） | stats/metrics 显示加密且权限已加固 |  |  |

## 网络 / 联邦证据

| 场景 | 所需证明 | 产物/链接 | 状态 |
| --- | --- | --- | --- |
| DHT ContactCard 发布/查找 | 联邦 smoke 报告/日志 |  |  |
| Mailbox 跨节点推送/取回 | 联邦 smoke 报告/日志 |  |  |
| 节点故障恢复 | chaos smoke 报告/日志 |  |  |
| 短时 Mailbox 负载 | 带 `MESSAGE_COUNT` 的 load smoke 报告/日志 |  |  |
| 公网部署配置 | 已脱敏的 `config.json`、Caddy/反向代理配置、节点 URL |  |  |

## 安全 / 审计证据

| 项目 | 所需证明 | 产物/链接 | 状态 |
| --- | --- | --- | --- |
| 外部安全审计 | 审计报告和修复说明 |  |  |
| 密码学审查 | 核心/WASM/节点控制面审查说明 |  |  |
| 依赖复核 | Dependabot / dependency-review 状态 |  |  |
| 签名 / 公证 | 若为生产发行则需 macOS 公证和 Windows 签名证据 |  |  |
| SECURITY.md 审查 | 发布分支的联系方式/流程已验证 |  |  |
| 风险登记 | 已审查 `docs/RELEASE_RISK_REGISTER.md`；Critical/High 风险已修复或明确接受 |  |  |

## 风险登记摘要

- 风险登记副本/链接：
- 未决 Critical 风险：
- 未决 High 风险：
- 已接受的 Medium/High 风险及批准人：

## 本次发布已接受的已知限制

- 

## 最终决定

- 发布批准人：
- UTC 日期/时间：
- 备注：

## 本地证据采集辅助工具

运行发布门禁后，将本地报告汇总到一个目录：

```bash
RELEASE_VERSION=vX.Y.Z RELEASE_EVIDENCE_DIR=release-evidence ./scripts/release-evidence.sh
```

该辅助工具会在存在时复制已知报告文件，并写入 `release-evidence-index.json`，标记为 `complete` / `incomplete`。这不替代人工审查；请在上表中填写 CI 产物、发布产物、审计和批准链接。

## 预生产证据运行器

要获取本地预生产证据包，请运行：

```bash
RELEASE_VERSION=preprod-local ./scripts/release-preprod.sh
```

可选参数：

```bash
RUN_FULL=0 RUN_FUZZ_SMOKE=1 RUN_SQLCIPHER=1 ./scripts/release-preprod.sh
RUN_FUZZ_CAMPAIGN=1 FUZZ_CAMPAIGN_DURATION=3600 ./scripts/release-preprod.sh
RUN_FEDERATION=1 ./scripts/release-preprod.sh
```

该脚本会写入已知日志/报告，然后调用 `scripts/release-evidence.sh`。它是证据采集辅助工具，不替代发布负责人或安全审核人的签核。
