# 发布检查清单

本检查清单是 LM Talk 发布候选版本的当前门禁。通过它是进入生产发布的必要条件，但不是充分条件：项目仍需完成生产级 DHT/relay 验证、长时 fuzz 活动、网络混沌/负载测试和外部安全审计，方可声称已达到生产就绪。

## 自动门禁

从仓库根目录运行发布检查。GitHub Actions `CI` 工作流会在推送和拉取请求时运行 `./scripts/release-check.sh quick`，因此该门禁也在远程环境中强制执行：

```bash
./scripts/release-check.sh quick
```

依赖漏洞检查应单独运行（GitHub Actions 中的 `dependency-audit` 任务在推送和拉取请求时执行；`dependency-review` 也会检查拉取请求中的依赖差异）：

```bash
./scripts/check-audit.sh
```

在生产签核前，运行风险登记门禁。该门禁故意不包含在快速 CI 中，因为开发阶段允许存在未决风险，但它对生产发布来说是一个否决门禁：

```bash
./scripts/release-risk-gate.sh
```

`./scripts/check-audit.sh` 中当前的审计例外范围很窄：`hickory-proto` 的 advisory 被忽略，是因为它来自未启用的可选 `libp2p` DNS/mDNS 依赖元数据，而 LM Talk 仅启用 TCP/noise/yamux/request-response；`paste` 被忽略，是因为它是通过 `libp2p-tcp` 间接引入的 Linux netlink proc-macro 警告。每当 `libp2p` 升级或启用 DNS/mDNS 功能时，必须重新评估这些例外。

对于一个更慢但包含完整 Cargo 工作区测试套件的本地门禁：

```bash
./scripts/release-check.sh full
```

要额外执行每个 fuzz harness 的短时 smoke 运行：

```bash
./scripts/release-check.sh fuzz-smoke
```

该脚本当前覆盖：

- Rust 格式检查（`cargo fmt --check`）。
- `lm_core` 的单元测试/端到端测试/属性测试/测试向量覆盖。
- `lm_node` 库和二进制测试。
- 节点端到端流程，包括 HTTP 控制面和 Mailbox 压力/故障恢复。
- fuzz harness 编译检查：`core_imports`、`node_dht_rpc`、`node_control_request`；`fuzz-smoke` 模式还会短暂启动每个目标。
- Web 类型检查、生产构建和 Playwright 端到端测试。

## 基于标签的原生节点产物

当推送匹配 `v*` 的 Git 标签，或手动触发具有标签名的工作流时，`.github/workflows/release-node.yml` 会构建发布原生 `lm_node` 二进制包。工作流会构建并发布：

- `lm_node-linux-x86_64.tar.gz`
- `lm_node-linux-x86_64-sqlcipher.tar.gz`
- `lm_node-macos-x86_64.tar.gz`
- `lm_node-macos-arm64.tar.gz`
- `lm_node-windows-x86_64.zip`

每个归档都包含 `lm_node` 二进制、关键部署/安全文档和 `RELEASE_INFO.txt`，其中记录源码提交、构建时间、Rust 工具链详情和二进制 SHA256。GitHub Release 还包括每个产物的 `.sha256` 文件和合并的 `SHA256SUMS.txt`。

发布工作流完成后，在分享标签前验证已发布产物和 SQLCipher 发布 smoke 证据：

```bash
./scripts/release-verify.sh v0.1.0
```

该命令会下载发布产物，验证 `SHA256SUMS.txt`，验证每个平台的 `.sha256` 文件，并检查归档的 SQLCipher smoke 报告是否证明 SQLCipher 产物的 `state_db` 指标已加密。

若要将已发布产物验证纳入自动化证据包：

```bash
RUN_RELEASE_ASSET_VERIFY=1 RELEASE_TAG_VERIFY=v0.1.0 RELEASE_VERSION=v0.1.0 ./scripts/release-preprod.sh
```

证据采集器会将 `release-asset-verify-report.json` 与常规的 release-check、fuzz、SQLCipher、联邦和风险登记门禁报告一起归档。

从当前提交剪切发布候选版本：

```bash
git tag -a v0.1.0 -m "LM Talk node v0.1.0"
git push origin v0.1.0
```

在 Linux 上构建目标后进行本地 artifact smoke 检查：

```bash
cargo build --locked --release -p lm_node --target x86_64-unknown-linux-gnu
python3 scripts/release-package.py \
  --target x86_64-unknown-linux-gnu \
  --package-name lm_node-linux-x86_64 \
  --out-dir dist
```

当前信任告警：自动化产物尚未获得 macOS notarization 或 Windows 代码签名。在生产信任发布渠道之前，应视签名/公证为必需。

## 仍未完成的人工发布阻塞项

在这些项未明确完成并提供证据前，不应将项目标记为生产就绪：

- 生产级 DHT/Kademlia 路由/查询稳健性和公开部署模型。
- Relay/TURN 替代方案不可成为硬性中心依赖的策略。
- 具有已保存语料库和崩溃分类的长时 fuzz 活动，超出 harness 编译检查。
- 真实网络混沌/负载测试：延迟、丢包、重连、畸形/恶意节点、持续 Mailbox/DHT 负载。
- 对核心密码学、Web/WASM 绑定、节点控制面和部署指南的外部安全审计。
- 原生节点 SQLCipher 数据库加密由 `lm_node/sqlcipher` 功能实现并由 `./scripts/check-sqlcipher.sh` 覆盖；在生产就绪发布前，必须归档 release workflow 的 `lm_node-linux-x86_64-sqlcipher-smoke` 产物，或等效部署运行，证明所选发布产物已用 `sqlcipher` 构建、以 `state_db_encryption_mode=sqlcipher` 启动，并报告已加密的 state DB 指标。JSON `state_file` 仅作为兼容/快照路径。
- 超出备份合并启发式的多设备同步与回执状态协调。

## 发布候选证据保留

每个发布候选应使用 `docs/RELEASE_EVIDENCE.md` 作为结构化证据索引。

在称某个节点构建为生产就绪前，还应为任何配置的状态持久化模式归档证据：

- `state_db`：`./scripts/check-sqlcipher.sh` 输出以及 release workflow 中的 `lm_node-linux-x86_64-sqlcipher-smoke` 产物（或等效部署证据），其中包含 `/control/stats` 和 `/control/metrics` 检查结果，确认 `state_db.encryption_mode=sqlcipher`、`state_db_encrypted=true`，以及 `lm_node_state_db_encrypted 1` 对应于该发布产物/配置。
- `state_file`：`/control/stats` 和 `/control/metrics` 显示 `state_file.encrypted=true` / `lm_node_state_file_encrypted 1`，并且 `state_file.permissions_hardened=true`；同时保留密码文件权限检查证据。

每个发布候选还应归档：

- `./scripts/release-check.sh full` 的输出。
- `./scripts/check-sqlcipher.sh` 的输出/产物、手动 SQLCipher Smoke 工作流和 release workflow 的 `lm_node-linux-x86_64-sqlcipher-smoke` 产物（如果 SQLCipher state DB 加密属于该发布）。
- fuzz 活动命令、持续时间、语料/崩溃产物和分类笔记。使用 `./scripts/fuzz-campaign.sh` 生成 JSON 活动报告以及每个目标的日志/语料/产物目录。
- 网络/负载测试报告和拓扑。
- 安全审计包（`docs/EXTERNAL_AUDIT_PACKET.md`）、审计报告和修复说明。
- 确认 RELEASE.md 联系/流程在发布分支中是最新的。
- 验证使用的构建产物哈希和部署配置。
- 每个原生产物的签名/公证证据（`*-signing-evidence.json`）以及 `docs/RELEASE_SIGNING.md` 的审查；如果 macOS/Windows 生产发行报告不完整，则仍视为否决。
- `./scripts/check-audit.sh` / CI `dependency-audit` 的输出。
- `./scripts/release-risk-gate.sh` 的输出，证明每个非低残余风险都有负责人、证据要求、证据链接和发布决策；`./scripts/release-preprod.sh` 会归档 `risk-register-gate.log` 和 `risk-register-gate-report.json`，并在 `release-evidence-index.json` 中记录机器可读的 `production_gate.risk_register_gate_status` / 计数。
- 对于拉取请求，CI `dependency-review` 对新增易受攻击依赖的状态。
- 对 Dependabot 生成的依赖更新 PR 的审查状态（`cargo`、Web npm 和 GitHub Actions）。
