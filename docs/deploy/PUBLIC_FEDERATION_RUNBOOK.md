# 公网联邦运行手册

本运行手册定义了如何部署和验证真实的公共 LM Talk 联邦。它是生成 `docs/PRODUCTION_READINESS.md` 和 `docs/RELEASE_SIGNOFF.md` 所需公网证据的操作流程。

## 目标

运行至少三个公共 `lm_node` 实例，提供：

- HTTPS 控制面 / Mailbox / DHT 端点。
- 明文 `state_db` 存储，磁盘静态保护由整盘加密（LUKS/dm-crypt）承担。
- 跨节点快照同步。
- ContactCard、PreKey、MailboxHint 和 PublicPeer 记录的 DHT 发布/查找。
- 跨节点 Mailbox 推送/取回/确认。
- 适合作为发布证据的指标和日志。

## 最低拓扑

| 节点 | 示例域名 | 角色 | 必需能力 |
| --- | --- | --- | --- |
| A | `node-a.example.com` | bootstrap + mailbox + DHT | `Bootstrap`, `Dht`, `Mailbox` |
| B | `node-b.example.com` | mailbox + DHT | `Dht`, `Mailbox` |
| C | `node-c.example.com` | mailbox + DHT | `Dht`, `Mailbox` |

每个节点应在 `sync_peers` 中列出另外两个节点。

## 部署前检查清单

- [ ] DNS A/AAAA 记录指向每台主机。
- [ ] HTTPS 证书已签发并支持自动续期。
- [ ] `cors_allow_origins` 只包含预期的 Web 源。
- [ ] 每个节点使用唯一 `peer_id`。
- [ ] 每个节点使用唯一控制令牌。
- [ ] 可能时为不同同步对等节点配置不同的令牌。
- [ ] `/data` 持久卷已备份并受监控。
- [ ] 主机防火墙仅公开 `80/443`；`8787` 通过反向代理私有访问。

## 部署步骤

1. 从 `deploy/lm-node-public` 开始，或根据实际情况调整 `deploy/lm-node-federation`。
2. 创建 secret：

```bash
openssl rand -base64 32 > secrets/control-token
chmod 600 secrets/*
```

3. 配置 `config.json`：

```json
{
  "bind": "0.0.0.0:8787",
  "peer_id": "node-a",
  "state_db": "/data/lm-node.sqlite3",
  "control_token_file": "/run/secrets/control-token",
  "cors_allow_origins": ["https://YOUR_WEB_ORIGIN"],
  "sync_peers": [
    { "url": "https://node-b.example.com", "peer_id": "node-b", "token_file": "/run/secrets/node-b-token" },
    { "url": "https://node-c.example.com", "peer_id": "node-c", "token_file": "/run/secrets/node-c-token" }
  ]
}
```

4. 启动：

```bash
docker compose up -d
```

## 验证命令

设置变量：

```bash
export NODE_A=https://node-a.example.com
export NODE_B=https://node-b.example.com
export NODE_C=https://node-c.example.com
export TOKEN_A=...
export TOKEN_B=...
export TOKEN_C=...
```

健康与统计：

```bash
curl -fsS "$NODE_A/health"
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/control/stats" | tee node-a-stats.json
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/control/metrics" | tee node-a-metrics.txt
```

DHT 维护：

```bash
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/dht/maintenance?factor=3&limit=8&max_targets=8" | tee node-a-dht-maintenance.json
```

快照同步演练：

```bash
curl -fsS -H "authorization: Bearer $TOKEN_A" "$NODE_A/sync/snapshot" > node-a-snapshot.json
curl -fsS -H "authorization: Bearer $TOKEN_B" -H 'content-type: application/json' \
  -d "{\"snapshot\":$(cat node-a-snapshot.json)}" \
  "$NODE_B/sync/import" | tee node-b-import.json
curl -fsS -H "authorization: Bearer $TOKEN_B" "$NODE_B/sync/status" | tee node-b-sync-status.json
```

## 所需发布证据

每次公网联邦运行应归档：

- 每个节点的脱敏 `config.json`。
- 每个节点的反向代理配置。
- 每个节点的 `/health` 输出。
- 每个节点的 `/control/stats` 和 `/control/metrics`。
- DHT 维护输出。
- 快照导出/导入输出。
- Mailbox 推送/取回/确认演练输出。
- 节点故障恢复演练输出。
- 包含消息数量、持续时间、失败和指标的负载测试报告。
- 验证期间的日志。

## 运营演练

### 节点故障恢复

- 停止节点 B。
- 向节点 A 推送 Mailbox 消息。
- 重启节点 B。
- 从节点 A 导入或等待同步。
- 验证节点 B 是否可以取回消息。

### ContactCard / PreKey / MailboxHint / PublicPeer 发现

- 通过 Web 客户端或节点辅助工具发布每种记录类型。
- 验证至少可从另一个节点查到。
- 验证过期/无效记录被拒绝。

### 令牌轮换

- 配置 `control_previous_tokens` 来保留旧令牌。
- 部署新令牌。
- 验证旧令牌在宽限期内仍可使用，并随后被移除。

## 公网联邦证据 Go / No-Go

若满足以下任一条件，则该公网联邦运行作为生产证据为 **NO-GO**：

- 任何节点缺少 HTTPS。
- 任何控制端点接受未经认证的非健康请求。
- 节点间快照同步失败。
- 跨节点 Mailbox 推送/取回失败。
- DHT ContactCard/PreKey 发布/查找跨节点失败。
- 日志显示重复 panic 或不受限的速率限制/配额失败。

## 报告模板

| 项目 | 产物/链接 | 状态 | 备注 |
| --- | --- | --- | --- |
| 节点 A 统计/指标 |  |  |  |
| 节点 B 统计/指标 |  |  |  |
| 节点 C 统计/指标 |  |  |  |
| DHT 发布/查找 |  |  |  |
| Mailbox 推送/取回/确认 |  |  |  |
| 故障恢复 |  |  |  |
| 负载测试 |  |  |  |
| 日志 |  |  |  |
