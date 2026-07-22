# LM Talk MVP 计划

本文记录当前功能目标下的 MVP 能力范围。当前目标聚焦“可用的去中心化端到端加密即时通讯”，不把第三方审计、长期公网报告或平台签名作为阻塞项。

## MVP 已覆盖能力

1. 身份创建、登录、导入和本地身份管理。
2. 身份备份、完整数据备份、导入合并和覆盖。
3. 联系人名片、好友请求/响应、指纹核验和拉黑。
4. 默认 strict E2EE 策略：已核验联系人 + sealed slot 收发。
5. 设备证书、设备撤销和 per-device sealed slot。
6. PreKey 发布/获取和安全会话建立。
7. 单聊消息、回执、Outbox 和 Mailbox 离线投递。
8. 文件包加密发送、Mailbox 收取、手动解密和下载。
9. 群聊 fanout、Sender Key、群邀请和群事件。
10. DHT 发现：ContactCard、PreKey、MailboxHint、PublicPeer。
11. Native `lm_node` 控制面、Mailbox、DHT、snapshot sync 和 metrics。
12. Docker 三节点 federation smoke/chaos/load 测试。
13. 诊断报告、strict E2EE 预检报告和脱敏摘要。

## 当前仍可打磨的功能

- Web 聊天体验细节：消息状态、失败提示、附件展示和搜索。
- strict E2EE 修复流程：更少跳转、更清晰的“一键修复”。
- Docker federation 测试易用性：减少环境依赖，完善失败提示。
- node-admin 运维界面：更直观展示 federation、DHT 和 Mailbox 状态。
- 文档持续中文化和去重。

## 不作为当前目标阻塞项

以下内容可作为未来生产发行增强，但不影响当前功能目标完成度：

- 第三方安全审计报告。
- 长时间 fuzz campaign 和 crash triage。
- 长时间公网 federation chaos/load 证据。
- 真实公网部署报告。
- macOS notarization / Windows code signing。
- 风险登记 owner / release decision / evidence 签核。

## 推荐验收命令

```bash
./scripts/release-check.sh quick
```

## 完成度估计

- 功能目标完成度：约 98%。
- 可用 Demo / MVP：约 98%。

剩余工作主要是体验、文档和部署便利性打磨。
