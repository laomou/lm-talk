# LM Talk 文档索引

本文档是 `docs/` 的入口。优先阅读顺序：

1. [设计总览](DESIGN.md)：系统目标、架构、协议边界和当前实现状态。
2. [MVP 计划](MVP_PLAN.md)：当前 MVP 范围和长期工作。
3. [安全模型](SECURITY_MODEL.md)：安全目标、非目标和已知限制。
4. 需要实现或排查某个协议时，再阅读下方对应规格。

## 架构与计划

| 文档 | 内容 |
|---|---|
| [DESIGN.md](DESIGN.md) | 总体设计、身份/好友/消息/群聊/节点/存储/安全边界。 |
| [MVP_PLAN.md](MVP_PLAN.md) | MVP 能力轨道和长期增强方向。 |
| [TODO.md](TODO.md) | 尚未完成或需要继续设计的事项。 |

## 安全与数据

| 文档 | 内容 |
|---|---|
| [SECURITY_MODEL.md](SECURITY_MODEL.md) | 安全目标、非目标和威胁边界。 |
| [IDENTITY_SPEC.md](IDENTITY_SPEC.md) | 身份种子、密钥派生、UserID 和提示词归一化。 |
| [BACKUP_SPEC.md](BACKUP_SPEC.md) | 身份备份包格式、Argon2id 标准路径和 WASM 兼容路径。 |
| [STORAGE_SPEC.md](STORAGE_SPEC.md) | Web IndexedDB、本地加密字段和迁移要求。 |

## 协议规格

| 文档 | 内容 |
|---|---|
| [CONTACT_SPEC.md](CONTACT_SPEC.md) | 联系人名片、签名字段、信任等级和指纹展示。 |
| [FRIEND_SPEC.md](FRIEND_SPEC.md) | 好友请求/响应格式和投递方式。 |
| [MESSAGE_SPEC.md](MESSAGE_SPEC.md) | Direct envelope、crypto id、AAD、ACK 和回执边界。 |
| [GROUP_SPEC.md](GROUP_SPEC.md) | MVP 群聊 fanout、Sender Key 和群事件边界。 |
| [PUBLIC_PEER_SPEC.md](PUBLIC_PEER_SPEC.md) | Public Peer announce、Mailbox 消息和反滥用要求。 |
| [NETWORK_SPEC.md](NETWORK_SPEC.md) | 网络策略、控制面鉴权和跨设备部署方式。 |

## 节点部署

| 文档 | 内容 |
|---|---|
| [NODE_CONFIG.md](NODE_CONFIG.md) | `lm_node serve-control` 配置文件、字段、备份恢复和运维说明。 |
| [examples/lm-node.config.example.json](examples/lm-node.config.example.json) | 节点配置示例。 |

## 测试向量

协议规格引用的固定测试向量位于仓库根目录的 `test-vectors/`。当前覆盖身份、备份、联系人名片、好友请求和消息加密。

完整测试覆盖状态记录在 [DESIGN.md](DESIGN.md) 的“当前端到端测试覆盖”附录；`TODO.md` 只保留尚未完成的工作。

## 维护约定

- `DESIGN.md` 记录跨模块总设计和实现状态，不放过细字段定义。
- `*_SPEC.md` 记录稳定协议和数据格式，尽量短、准、可实现。
- `NODE_CONFIG.md` 只记录节点部署和运维细节。
- `TODO.md` 只记录未完成事项；已完成的测试覆盖应同步到 `DESIGN.md` 或对应规格。
