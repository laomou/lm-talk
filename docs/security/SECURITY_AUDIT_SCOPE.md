# 安全审查范围

本文列出建议审查的安全范围。当前功能目标不要求第三方审计报告；本文用于内部自查或未来外部审计。

## 安全目标

1. 单聊、文件和群聊内容端到端加密。
2. 通过 DHT、Mailbox 和 federation 实现去中心化发现与投递。
3. 使用联系人指纹、设备证书、设备撤销和 sealed slot 管理信任。
4. Web 本地数据加密保存。
5. strict E2EE 默认启用，核心内容路径 fail-closed。

## 核心协议范围

- 身份创建、备份、恢复和提示词归一化。
- ContactCard 签名、验签、指纹和设备证书。
- 好友请求/响应。
- DirectEnvelope 兼容路径。
- X3DH PreKey 和 signed one-time prekey。
- Double Ratchet 状态、重放、乱序和 skipped key。
- Group Sender Key、群事件和群权限。
- 文件包加密和 hash 校验。
- MessageReceipt 与 delivery/read 状态。
- 版本、前缀、过期和大小限制。

## Web / WASM 范围

- WASM API 边界。
- Web RNG 身份生成。
- IndexedDB 加密、迁移、删除身份和重加密。
- 完整数据备份导入/导出。
- strict E2EE 默认策略和阻断路径。
- sealed slot、多设备证书和撤销 UI。
- ContactCard 更新、ACK、stale retry 和 DHT 刷新。
- 修复控制消息例外：ContactCard/device-cert 更新和 device revoke。

## 原生节点范围

- HTTP 控制面解析和鉴权。
- CORS、token、previous token 和限流。
- Mailbox push/take/ack、TTL、配额和去重。
- PreKey 发布/获取/消费。
- DHT record 校验和查询。
- snapshot export/import。
- state_db 持久化和权限。
- metrics 不泄漏敏感载荷。

## 部署范围

- Docker federation 模板。
- Caddy / TLS / secret / CORS 配置。
- 端口暴露。
- 节点间 sync_peers。
- federation smoke/chaos/load 脚本。

## 威胁模型

- 恶意 Mailbox / DHT / relay 节点。
- 恶意或被攻陷联系人。
- 丢失设备和撤销竞态。
- replay、乱序、重复、过期、畸形和超大对象。
- sealed slot 降级到 fallback。
- DHT poisoning 和恶意 closer node。
- 控制面 token 泄漏和资源耗尽。

## 非目标

- 不提供匿名性。
- 不隐藏全部元数据。
- 不防御完全控制用户运行时的恶意软件。
- 用户丢失身份备份和提示词后无法恢复。

## 建议交付物

- 发现列表。
- 严重性和影响路径。
- 复现步骤。
- 修复建议。
- 验证命令。
- 残余风险说明。
