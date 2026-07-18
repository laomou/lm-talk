# 公网部署报告模板

在运行真实公共 LM Talk 联邦演练后使用此模板。它应链接到 `docs/RELEASE_EVIDENCE.md` 和 `docs/RELEASE_SIGNOFF.md`，在任何生产就绪声明前完成。

## 部署身份

- 报告 ID：
- UTC 日期/时间：
- 运营人员：
- 发布/标签：
- 提交 SHA：
- Web 来源：
- 节点产物：
- 使用的 SQLCipher 产物：是/否

## 拓扑

| 节点 | 域名 | 区域/供应商 | Peer ID | 角色 | 版本/提交 | SQLCipher 模式 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A |  |  |  | bootstrap + DHT + mailbox |  |  |  |
| B |  |  |  | DHT + mailbox |  |  |  |
| C |  |  |  | DHT + mailbox |  |  |  |

## 配置证据

附上脱敏副本或链接：

- [ ] 节点 A `config.json`
- [ ] 节点 B `config.json`
- [ ] 节点 C `config.json`
- [ ] 每个节点的反向代理 / TLS 配置
- [ ] 防火墙/安全组摘要
- [ ] CORS 允许列表
- [ ] 令牌轮换计划
- [ ] 备份/恢复计划

## 健康与指标

| 节点 | `/health` 归档 | `/control/stats` 归档 | `/control/metrics` 归档 | 状态 |
| --- | --- | --- | --- | --- |
| A |  |  |  |  |
| B |  |  |  |  |
| C |  |  |  |  |

必需检查：

- [ ] 每个节点都提供 HTTPS。
- [ ] 仅 `/health` 为未认证访问。
- [ ] 未提供或错误的 Bearer token 时，认证端点被拒绝。
- [ ] 指标中不包含明文消息、身份备份、令牌或解密载荷。

## SQLCipher / 持久化证据

对于每个使用 SQLCipher 的节点：

| 节点 | `state_db.encryption_mode` | `state_db.encrypted` | `lm_node_state_db_encrypted` | 错误密码测试 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A |  |  |  |  |  |
| B |  |  |  |  |  |
| C |  |  |  |  |  |

附上：

- [ ] `sqlcipher-deploy-smoke.log` 或等效输出
- [ ] `/control/stats` 证据
- [ ] `/control/metrics` 证据
- [ ] 错误密码 fail-closed 输出

## DHT 验证

| 记录类型 | 发布自 | 查找自 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| ContactCard |  |  |  |  |
| PreKey |  |  |  |  |
| MailboxHint |  |  |  |  |
| PublicPeer |  |  |  |  |

必需检查：

- [ ] 有效记录至少可从另一个节点查到。
- [ ] 无效/不匹配记录被拒绝。
- [ ] DHT 维护/复制输出已归档。
- [ ] 没有健康对等节点被重复隔离。

## Mailbox 验证

| 场景 | 证据 | 状态 |
| --- | --- | --- |
| 向节点 A 推送签名消息 |  |  |
| 从节点 A 取回 |  |  |
| 将快照导入节点 B |  |  |
| 从节点 B 取回恢复消息 |  |  |
| 确认交付并验证 tombstone/状态 |  |  |
| 短时负载下配额/限流行为 |  |  |

## 联邦 / 混沌 / 负载

| 演练 | 命令/报告 | 持续/计数 | 结果 | 备注 |
| --- | --- | --- | --- | --- |
| 基础 smoke |  |  |  |  |
| 节点故障恢复 |  |  |  |  |
| 短时负载 |  |  |  |  |
| 较长负载/分区测试 |  |  |  |  |

附上 `federation-report.json`、日志和任何负载测试摘要。

## 客户端/Web 验证

| 流程 | 证据 | 状态 |
| --- | --- | --- |
| Web 客户端连接所有节点 |  |  |
| ContactCard DHT 发布/查找 |  |  |
| PreKey DHT 发布/查找 |  |  |
| Mailbox 发送/接收 |  |  |
| 严格 E2EE 模式发送/接收 |  |  |
| 设备证书更新/ACK 收敛 |  |  |
| 自同步回执/发件箱摘要 |  |  |

## 事件与异常

列出任何意外错误、警告、隔离对等节点、速率限制峰值、Mailbox 投递失败或 DHT 查找失败。

| UTC 时间 | 节点/客户端 | 症状 | 根因 | 解决 | 跟进行动 |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |

## Go / No-Go 结论

- 决策：`通过` / `不通过`
- 阻塞问题：
- 已接受风险：
- 需要后续工单：
- 运营人员签核：
- 安全审核人签核：
- UTC 日期/时间：

若任何节点缺少 HTTPS、SQLCipher 证据缺失（对于 SQLCipher 部署）、跨节点 Mailbox 恢复失败、DHT ContactCard/PreKey 发现失败，或验证期间日志显示重复 panic，则部署报告作为生产证据为 **不通过**。
