# 发布签核模板

本文是可选模板，用于记录一次演示版、内部测试版或正式发行的最终确认。当前功能目标不要求填写完整生产签核。

## 发布候选

| 项目 | 内容 |
| --- | --- |
| 版本 / tag |  |
| commit |  |
| 分支 |  |
| UTC 时间 |  |
| 负责人 |  |

## 必要检查

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 工作区干净 | `git status --short` |  |
| 快速检查通过 | `./scripts/release-check.sh quick` |  |
| Web E2E 通过 | Playwright 结果 |  |
| Docker federation 通过 | `federation-report.json` |  |
| 产物校验和 | `SHA256SUMS.txt` / `.sha256` |  |

## 功能确认

| 功能 | 状态 | 备注 |
| --- | --- | --- |
| 身份创建/登录/备份 |  |  |
| 联系人添加与指纹核验 |  |  |
| 单聊 E2EE |  |  |
| 文件加密发送/接收 |  |  |
| 群聊与群事件 |  |  |
| strict E2EE 默认策略 |  |  |
| sealed slot 多设备路径 |  |  |
| Mailbox 离线投递 |  |  |
| DHT ContactCard/PreKey/MailboxHint |  |  |
| Native node federation |  |  |

## 可选生产增强

以下项目不是当前目标阻塞项；如需要公开生产发行再填写：

| 项目 | 证据 | 状态 |
| --- | --- | --- |
| 外部安全审计 |  |  |
| 长时间 fuzz |  |  |
| 长时间公网 federation |  |  |
| 公网部署报告 |  |  |
| macOS notarization |  |  |
| Windows signing |  |  |
| 风险登记签核 |  |  |

## 结论

- 决策：`GO` / `NO-GO`
- 范围：`演示版` / `内部测试` / `公开发行`
- 已知限制：
- 后续事项：
- 批准人：
- 时间：
