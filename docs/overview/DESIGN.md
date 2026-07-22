# LM Talk 设计文档

本文是 LM Talk 当前实现的总设计说明。目标是说明系统如何成为一个**去中心化、端到端加密的即时通讯应用**。更细的字段格式见 `docs/protocol/`，节点部署见 `docs/deploy/`。

## 1. 项目定位

LM Talk 是一个本地身份自持、好友双向确认、内容端到端加密、节点只转发密文的即时通讯系统。

核心原则：

- 不依赖手机号、邮箱或中心账号服务器。
- 身份由本地随机种子生成。
- 恢复身份依赖身份备份和提示词。
- 好友关系必须双方确认。
- 消息、文件和群聊内容在发送端加密、接收端解密。
- 节点只保存密文、签名控制对象和公开 DHT 记录。
- 本地安全策略由客户端执行。

## 2. 总体架构

```text
Web UI / node-admin
        │
        ▼
lm_wasm / TypeScript bindings
        │
        ▼
lm_core：身份、联系人、消息、文件、群聊、设备、PreKey、Ratchet
        │
        ├── Web IndexedDB 本地加密存储
        └── lm_node：Mailbox / DHT / PreKey / snapshot / metrics
```

## 3. 当前模块状态

| 模块 | 当前状态 |
| --- | --- |
| `lm_core` | 已实现身份、备份、ContactCard、好友请求、DirectEnvelope、X3DH、Double Ratchet、文件包、设备证书/撤销、群 Sender Key、回执、大小限制和测试向量。 |
| `lm_wasm` | 已暴露 Web 需要的大部分协议和密码学接口。 |
| Web app | 已具备注册、登录、联系人、聊天、文件、群聊、同步、备份、诊断和 strict E2EE 修复流程。 |
| `lm_node` | 已提供 HTTP 控制面、Mailbox、PreKey、DHT、snapshot sync、state_db、metrics 和 Docker federation 模板。 |
| node-admin | 已提供独立运维面板，用于查看节点健康、DHT、snapshot 和运行统计。 |

## 4. 身份模型

每个用户身份由本地随机 `identity_seed` 生成：

```text
identity_seed -> identity keypair -> identity_public_key -> UserID
```

身份能力：

- Ed25519 身份签名。
- X25519 静态公钥。
- User ID 稳定派生。
- 身份备份包由提示词保护。
- Web 本地状态由身份派生密钥加密。

节点不签发、不托管、不恢复用户身份。

## 5. 联系人与信任

联系人名片 `lm-contact-card-v1:` 是签名身份声明，包含：

- User ID；
- 身份公钥；
- X25519 公钥；
- 显示名；
- 设备证书列表；
- 签名和时间字段。

本地信任状态不会被远端覆盖：

- 指纹已核验状态；
- 拉黑状态；
- 设备撤销状态；
- 已读回执策略；
- 本地安全策略。

好友关系通过请求/响应建立：

```text
Friend Request -> Friend Response -> Friend
```

## 6. Strict E2EE 默认策略

新身份默认启用 strict E2EE：

- 发送前要求联系人指纹已核验。
- 发送前要求目标活跃设备支持 sealed slot。
- 接收时要求联系人已核验。
- 接收时要求 sealed slot 入站。

核心阻断项：

- 未核验指纹；
- 所有已知设备已撤销；
- 缺少活跃设备；
- 活跃设备缺少 `device_box_public_key`；
- 收到非 sealed slot 入站内容。

非阻断提醒：

- ContactCard DHT 未刷新；
- 设备证书更新 ACK 待确认。

修复控制消息例外：

- ContactCard / device cert 更新；
- device revoke。

这些控制消息不携带聊天明文，用于修复或撤销信任状态，因此允许在 strict 状态未完全收敛时发送。

## 7. 多设备与 sealed slot

每个设备拥有设备证书，证书由身份签名，并包含：

- 设备签名公钥；
- `device_box_public_key`；
- Device ID。

发送时会为目标设备创建 per-device sealed slot。strict 模式下，若目标设备缺少 sealed slot 能力，则阻止内容发送。

设备丢失或废弃时，用户生成 `lm-device-revoke-v1:`，对方收到后停止信任对应设备。

旧身份或导入身份如果缺少本设备证书，登录或同步前会自动补齐，并向好友 fanout 新 ContactCard。

## 8. 消息与回执

单聊消息路径：

1. 好友建立。
2. PreKey / X3DH / Ratchet 建链。
3. 内容加密。
4. per-device envelope 封装。
5. WebRTC 或 Mailbox 投递。
6. 接收方解密并发送 Delivered / Read receipt。

Mailbox delivery id 和 signed MessageReceipt 用于恢复送达/已读状态。

## 9. 文件传输

文件以 `lm-file-package-v1:` 加密包传输。

特点：

- 文件内容加密后再发送。
- 接收后不自动下载。
- 危险文件名会提示。
- strict 模式下，文件发送、Mailbox 接收和手动解密均检查联系人信任。

## 10. 群聊

群聊采用成员 fanout：

- 无 Sender Key 时，为每个成员分别加密。
- 有 Sender Key 时，生成 group sender envelope 后 fanout。
- 群事件用于改名、加人、移除、管理员变更等。

strict 模式覆盖：

- 创建群聊前预检。
- 接受群邀请前预检。
- 发送群消息前检查。
- 生成群事件 fanout 前检查。
- 群详情页显示修复向导。

新成员不会自动收到历史消息。历史转移必须显式重新加密。

## 11. 去中心化网络

LM Talk 使用多种 best-effort 路径：

| 路径 | 说明 |
| --- | --- |
| WebRTC | 在线直连。 |
| Mailbox | 离线密文投递。 |
| DHT | 发现 ContactCard、PreKey、MailboxHint、PublicPeer。 |
| Snapshot sync | 节点之间同步运营状态。 |
| Outbox | 客户端本地重试队列。 |

节点只保存：

- 签名 Mailbox 密文对象；
- DHT 公开记录；
- PreKey 公开对象；
- snapshot 运营状态。

节点不保存消息明文和用户私钥。

## 12. Native node

`lm_node serve-control` 提供：

- `/health`
- `/control/stats`
- `/control/metrics`
- Mailbox push/take/ack/status
- PreKey publish/get/status
- DHT record store/find/closest
- DHT replication / routing refresh
- snapshot export/import

节点配置见 `docs/deploy/NODE_CONFIG.md`。

## 13. 本地存储

Web 使用 IndexedDB 分表保存状态。敏感字段加密，包括：

- 消息文本和 envelope；
- 联系人名片和显示名；
- 群状态；
- outbox 载荷；
- Ratchet 状态；
- 好友请求和群邀请原文；
- 同步服务地址和令牌。

为了索引和 UI，User ID、message id、group id、状态、时间戳等最小字段保持明文。

删除本地身份时，会删除该身份作用域的数据。

## 14. 诊断与报告

Web 提供：

- 诊断报告；
- strict E2EE readiness 完整报告；
- strict E2EE 脱敏摘要；
- DHT 操作历史导出；
- 同步恢复历史导出。

诊断报告不得包含提示词、私钥、身份备份全文、消息明文或控制面 token。

## 15. 测试

常用检查：

```bash
./scripts/release-check.sh quick
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/web run test:e2e
```

覆盖范围：

- core 单元测试和属性测试；
- 测试向量；
- WASM smoke；
- Web E2E；
- node 控制面 E2E；
- Docker federation smoke/chaos/load。

## 16. 当前完成度

| 目标 | 估计 |
| --- | --- |
| 可用 Demo / MVP | 约 98% |
| 功能目标整体 | 约 98% |

仍可继续打磨：

- Docker federation 易用性；
- strict E2EE 修复体验；
- 消息状态展示；
- node-admin 运维体验；
- 文档去重和中文化。

## 17. 当前不作为阻塞项

以下事项可作为未来生产发行增强，但不再阻塞当前功能目标：

- 第三方安全审计报告。
- 长时间公网 federation chaos/load 证据。
- 真实公网部署报告。
- macOS notarization / Windows code signing。
- 风险登记 owner / release decision / evidence 完整签核。
