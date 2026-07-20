# 外部安全审计交付包

本文是未来外部安全审计的入口包。当前功能目标不要求第三方审计报告；本文保留为可选生产增强资料。

## 审计目标

LM Talk 是一个去中心化端到端加密即时通讯系统，包含：

- Web/WASM 客户端；
- Rust 核心协议库；
- 原生 `lm_node` 节点；
- Mailbox / DHT / federation 部署模板；
- strict E2EE 默认策略、设备证书、设备撤销和 sealed slot。

## 审计标识

需要实际审计时再填写：

| 字段 | 值 |
| --- | --- |
| 审计 commit |  |
| 审计 tag |  |
| 审计方 |  |
| 证据目录 |  |
| 审计时间 |  |
| 负责人 |  |

## 推荐阅读顺序

1. `docs/overview/DESIGN.md`：总体架构和实现状态。
2. `docs/security/SECURITY_MODEL.md`：安全目标和非目标。
3. `docs/security/CRYPTO_REVIEW_NOTES.md`：密码学边界和 strict E2EE 控制消息例外。
4. `docs/security/SECURITY_AUDIT_SCOPE.md`：建议审查范围。
5. `docs/testing/PROTOCOL_STABILITY.md`：协议对象稳定性。
6. `docs/testing/TEST_VECTOR_COVERAGE.md`：测试向量覆盖。
7. `docs/deploy/NODE_CONFIG.md`：节点配置和运维。
8. `docs/deploy/PUBLIC_FEDERATION_RUNBOOK.md`：联邦部署运行说明。

## 代码地图

| 领域 | 路径 | 审查重点 |
| --- | --- | --- |
| 核心协议 | `crates/lm_core/src` | 身份、ContactCard、好友、消息、文件、群聊、Ratchet、PreKey、设备。 |
| WASM | `crates/lm_wasm/src` | JS/WASM 边界、错误映射、sealed slot API。 |
| Web 客户端 | `apps/web/src` | strict E2EE、IndexedDB、备份、自同步、UI 风险提示。 |
| 原生节点 | `crates/lm_node/src` | 控制面、Mailbox、DHT、PreKey、snapshot、metrics。 |
| 部署 | `deploy/` | Docker、Caddy、secret、CORS、节点拓扑。 |
| 测试 | `tests/`, `test-vectors/` | Web E2E、节点 E2E、协议向量、fuzz harness。 |

## 建议命令

```bash
./scripts/release-check.sh quick
./scripts/check-audit.sh
./scripts/fuzz-smoke.sh
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json deploy/lm-node-federation/run-all.sh
```

## 高优先级问题

- 节点是否能看到消息或文件明文？
- 未核验联系人是否能绕过 strict 入站/出站策略？
- sealed slot 是否可能静默降级到 fallback？
- 设备撤销是否优先于旧 ContactCard？
- ContactCard / PreKey / PublicPeer / DHT 记录是否绑定正确 key namespace？
- Mailbox 是否只保存签名密文对象？
- Web 本地数据是否避免明文长期落盘？
- 控制面 token、CORS、限流和大小限制是否足够防滥用？

## 审计输出建议

每个发现建议包含：

- 严重性；
- 受影响组件；
- 利用条件；
- 复现步骤；
- 建议修复；
- 验证命令；
- 残余风险说明。

审计发现可记录在 `docs/security/AUDIT_REMEDIATION_TRACKER.md`。
