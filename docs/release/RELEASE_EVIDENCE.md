# 发布证据索引

本文是可选的发布证据记录模板。当前功能目标不要求填写完整生产证据；需要归档某个可演示版本、Docker 部署测试或原生节点包时，可复制本模板到 issue / release notes 中填写。

## 发布候选

| 项目 | 内容 |
| --- | --- |
| 版本 / tag |  |
| commit |  |
| UTC 时间 |  |
| 负责人 |  |

## 功能检查

| 检查 | 命令 / 证据 | 状态 |
| --- | --- | --- |
| 快速发布检查 | `./scripts/release-check.sh quick` |  |
| 完整发布检查 | `./scripts/release-check.sh full` |  |
| Web E2E | `npm --prefix apps/web run test:e2e` |  |
| 测试向量 | `cargo test -p lm_core --test test_vectors` |  |
| 依赖审计 | `./scripts/check-audit.sh` |  |

## Docker / federation 证据

| 检查 | 证据 | 状态 |
| --- | --- | --- |
| basic smoke | `federation-report.json` |  |
| chaos smoke | `federation-report.json` |  |
| load smoke | `federation-report.json` |  |
| Mailbox push/take | 日志或报告 |  |
| DHT ContactCard publish/find | 日志或报告 |  |

## 原生节点产物

| 产物 | SHA256 / 链接 | 状态 |
| --- | --- | --- |
| Linux x86_64 |  |  |
| macOS x86_64 |  |  |
| macOS arm64 |  |  |
| Windows x86_64 |  |  |
| `SHA256SUMS.txt` |  |  |

## strict E2EE 证据

| 场景 | 证据 | 状态 |
| --- | --- | --- |
| 新身份默认 strict E2EE | Web smoke / 手动截图 |  |
| 联系人指纹核验 | Web smoke / 手动截图 |  |
| sealed slot 发送 | Web smoke / 向量 |  |
| 文件发送/接收 strict 策略 | Web smoke / 手动截图 |  |
| 群聊 strict 风险提示与阻断 | Web smoke / 手动截图 |  |
| strict E2EE 预检报告 | 下载的 JSON |  |

## 可选生产增强证据

这些不是当前目标阻塞项，仅在公开生产发行时填写：

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 外部安全审计 |  |  |
| 公网长期 chaos/load |  |  |
| 公网部署报告 |  |  |
| macOS notarization |  |  |
| Windows signing |  |  |
| 风险登记签核 |  |  |

## 结论

- 结论：`可演示` / `需修复` / `仅内部测试`
- 已知限制：
- 后续事项：
