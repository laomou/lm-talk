# 公网联邦运行手册

本文说明如何部署和验证 LM Talk 公网联邦节点。当前功能目标不要求真实公网长期报告；本文作为运维参考和未来公网部署说明保留。

## 目标

至少运行三个 `lm_node` 节点，提供：

- HTTPS 控制面、Mailbox 和 DHT 接口。
- 节点间 snapshot sync。
- ContactCard、PreKey、MailboxHint、PublicPeer 的 DHT 发布与查找。
- Mailbox push / take / ack。
- `/health`、`/control/stats`、`/control/metrics` 监控接口。

## 最小拓扑

| 节点 | 示例域名 | 角色 | 能力 |
| --- | --- | --- | --- |
| A | `node-a.example.com` | bootstrap + mailbox + DHT | `Bootstrap`, `Dht`, `Mailbox` |
| B | `node-b.example.com` | mailbox + DHT | `Dht`, `Mailbox` |
| C | `node-c.example.com` | mailbox + DHT | `Dht`, `Mailbox` |

每个节点应把其他两个节点配置到 `sync_peers`。

## 部署前检查

- DNS A/AAAA 已指向服务器。
- HTTPS 证书已签发并能自动续期。
- `cors_allow_origins` 只包含实际 Web 站点来源。
- 每个节点使用唯一 `peer_id`。
- 每个节点使用唯一 control token。
- secret 文件不是 symlink，权限建议 `0600`。
- 公网只暴露 80/443，`8787` 应留在反向代理或内网后面。
- `state_db` 所在磁盘或目录应由系统层加密保护。

## 部署步骤

1. 以 `deploy/lm-node-public` 或 `deploy/lm-node-federation` 为模板。
2. 生成密钥文件：

```bash
openssl rand -base64 32 > secrets/control-token
openssl rand -base64 32 > secrets/state-db-passphrase
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

4. 启动服务：

```bash
docker compose up -d --build
```

5. 验证本地三节点模板：

```bash
deploy/lm-node-federation/run-all.sh
```

## 验证命令

```bash
curl -fsS https://node-a.example.com/health
curl -fsS -H "authorization: Bearer $TOKEN" https://node-a.example.com/control/stats
curl -fsS -H "authorization: Bearer $TOKEN" https://node-a.example.com/control/metrics
```

应检查：

- 三个节点均健康。
- snapshot export/import 正常。
- ContactCard DHT 发布/查找正常。
- Mailbox push/take/ack 正常。
- metrics 中没有异常 4xx/5xx 激增。

## 运维演练

### 节点故障恢复

1. 停止一个节点。
2. 向其他节点 push Mailbox 消息。
3. 恢复节点。
4. 导入 snapshot 或等待自动同步。
5. 确认 Mailbox 消息可取回。

### DHT 发现

验证以下对象可发布并查找：

- ContactCard
- PreKey
- MailboxHint
- PublicPeer

### token 轮换

1. 新 token 加入 `control_previous_tokens`。
2. 客户端切换到新 token。
3. 观察无失败后移除旧 token。

## 报告模板

需要形成公网部署记录时，可使用 `docs/deploy/PUBLIC_DEPLOYMENT_REPORT_TEMPLATE.md`。该报告属于可选生产增强，不再作为当前功能目标阻塞项。
