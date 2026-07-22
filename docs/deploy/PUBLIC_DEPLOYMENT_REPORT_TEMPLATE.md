# 公网部署报告模板

本文是可选模板，用于记录一次真实公网或准公网 federation 部署。当前功能目标不要求填写本报告；当需要对外运营、团队交接或发布说明时，可复制本模板填写。

## 基本信息

| 项目 | 内容 |
| --- | --- |
| 报告 ID |  |
| 部署时间 UTC |  |
| 版本 / tag |  |
| commit |  |
| Web 来源 |  |
| 负责人 |  |

## 拓扑

| 节点 | 域名 / 地址 | Peer ID | 角色 | 版本 | 备注 |
| --- | --- | --- | --- | --- | --- |
| A |  |  | bootstrap + DHT + mailbox |  |  |
| B |  |  | DHT + mailbox |  |  |
| C |  |  | DHT + mailbox |  |  |

## 配置摘要

记录脱敏后的配置：

- `bind`：
- `state_db`：
- `control_token_file`：
- `cors_allow_origins`：
- `sync_peers`：
- 反向代理 / TLS：
- 防火墙 / 安全组：
- 备份路径：

## 健康与指标

| 节点 | `/health` | `/control/stats` | `/control/metrics` | 状态 |
| --- | --- | --- | --- | --- |
| A |  |  |  |  |
| B |  |  |  |  |
| C |  |  |  |  |

建议检查：

- 三个节点均返回健康状态。
- 认证接口拒绝错误 token。
- metrics 中没有明文消息、身份备份、token 或解密载荷。
- Mailbox/DHT 计数符合预期。

## DHT 验证

| 记录类型 | 发布节点 | 查找节点 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| ContactCard |  |  |  |  |
| PreKey |  |  |  |  |
| MailboxHint |  |  |  |  |
| PublicPeer |  |  |  |  |

## Mailbox 验证

| 场景 | 结果 | 证据 |
| --- | --- | --- |
| push 到节点 A |  |  |
| 从节点 A take |  |  |
| snapshot 导入节点 B |  |  |
| 从节点 B take |  |  |
| ack / delivery status |  |  |
| 配额 / 限流 |  |  |

## Federation smoke / chaos / load

记录：

| 测试 | 参数 | 结果 | 报告 |
| --- | --- | --- | --- |
| smoke |  |  |  |
| chaos |  |  |  |
| load | `MESSAGE_COUNT=` |  |  |

## Web 客户端验证

| 流程 | 结果 | 证据 |
| --- | --- | --- |
| 添加同步服务 |  |  |
| 发布/查找 ContactCard DHT |  |  |
| 好友请求 |  |  |
| 单聊消息 |  |  |
| 文件包 |  |  |
| 群聊 |  |  |
| strict E2EE 默认策略 |  |  |
| 设备证书更新 / ACK |  |  |
| 自同步 |  |  |

## 异常记录

| 时间 UTC | 节点/客户端 | 现象 | 原因 | 处理 | 后续 |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |

## 结论

- 结论：`通过` / `不通过` / `仅内部测试`
- 已知限制：
- 后续事项：
- 负责人：
- 时间：
