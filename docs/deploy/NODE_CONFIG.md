# LM Node 配置说明

`lm_node serve-control` 是 LM Talk 的原生节点控制面，用于提供 Mailbox、DHT、PreKey、snapshot sync 和运维指标。

## 快速启动

本机试用：

```bash
lm_node serve-control
```

对外监听并启用令牌和 SQLite 状态库：

```bash
lm_node serve-control \
  --bind 0.0.0.0:8787 \
  --control-token "$(cat /etc/lm-node/control.token)" \
  --state-db /var/lib/lm-node/state.sqlite3
```

推荐把配置写入 JSON：

```bash
lm_node serve-control --config-file node.json
```

也可以使用环境变量：

```bash
LM_NODE_CONFIG_FILE=node.json lm_node serve-control
```

优先级：

```text
命令行参数 > 环境变量 > 配置文件 > 默认值
```

敏感值建议使用环境变量或 `*_file` 字段，不要直接写进配置文件。

## 配置示例

示例文件：

```text
docs/examples/lm-node.config.example.json
```

最小配置示例：

```json
{
  "bind": "0.0.0.0:8787",
  "peer_id": "lm-node-a",
  "state_db": "/data/lm-node.sqlite3",
  "control_token_file": "/run/secrets/control-token",
  "cors_allow_origins": ["http://localhost:5173"],
  "sync_peers": [
    { "url": "http://node-b:8787", "peer_id": "lm-node-b", "token_file": "/run/secrets/node-b-token" }
  ]
}
```

## 主要字段

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `bind` | `127.0.0.1:8787` | 控制面监听地址。公网部署建议由反向代理提供 TLS。 |
| `peer_id` | `lm-node-dev` | 节点公开 peer id。 |
| `state_db` | 无 | SQLite 状态数据库，保存 Mailbox、PreKey、DHT、routing peer 和 snapshot 状态。 |
| `control_token` | 无 | 控制面 Bearer token。配置后除 `/health` 外都需要认证。 |
| `control_token_file` | 无 | 从文件读取控制面 token；建议权限 `0600`，拒绝 symlink。 |
| `control_previous_tokens` | `[]` | 短期保留旧 token，用于无停机轮换。 |
| `cors_allow_origins` | `[]` | Web / node-admin 允许跨域访问的 Origin。 |
| `sync_interval_seconds` | `0` | 自动 snapshot sync 周期；`0` 表示关闭。 |
| `sync_max_backoff_seconds` | `300` | snapshot sync 失败退避上限。 |
| `sync_peers` | `[]` | 其他节点控制面，用于 snapshot sync 和 DHT runner。 |
| `dht_replication_factor` | `3` | DHT record 复制目标数；`0` 可关闭复制。 |
| `dht_routing_refresh_limit` | `8` | 每次 FindNode 希望返回的节点数。 |
| `dht_routing_refresh_max_targets` | `8` | 每轮最多查询的 refresh target。 |
| `dht_peer_quarantine_consecutive_failures` | `5` | 连续失败达到阈值后，DHT runner 暂时跳过该 peer。 |
| `rate_limit_window_seconds` | `60` | 控制面基础限流窗口。 |
| `rate_limit_max_requests` | `600` | 每个窗口允许的最大请求数。 |
| `log_format` | `text` | 日志格式：`text` 或 `json`。 |
| `max_mailbox_bytes` | `10485760` | 节点保留的未 ACK Mailbox 总字节上限。 |
| `max_mailbox_bytes_per_user` | `2097152` | 单个收件用户未 ACK Mailbox 字节上限。 |
| `max_mailbox_messages_per_user` | `1000` | 单个收件用户未 ACK Mailbox 消息数上限。 |
| `mailbox_global_rate_limit_window_seconds` | `null` | Mailbox 全局 push 限流窗口。 |
| `mailbox_global_rate_limit_max_messages` | `null` | 全局窗口内最多 push 数。 |
| `mailbox_sender_rate_limit_window_seconds` | `null` | 按发送者限流窗口。 |
| `mailbox_sender_rate_limit_max_messages` | `null` | 单发送者窗口内最多 push 数。 |

## `sync_peers[]`

| 字段 | 说明 |
| --- | --- |
| `url` | 远端控制面 URL，目前以 HTTP(S) 控制面为主。 |
| `peer_id` | 可选；远端 PublicPeer id。配置后 DHT 复制会按 closest-k 计划选择目标。 |
| `token` | 远端控制面 token。 |
| `token_file` | 从文件读取远端 token；建议权限 `0600`。 |

## CLI / 环境变量映射

| 配置 | CLI | 环境变量 |
| --- | --- | --- |
| 配置文件 | `--config-file` | `LM_NODE_CONFIG_FILE` |
| 监听地址 | `--bind` | - |
| 节点 ID | `--peer-id` | - |
| 状态库 | `--state-db` | - |
| 控制令牌 | `--control-token` | `LM_NODE_CONTROL_TOKEN` |
| 控制令牌文件 | `--control-token-file` | `LM_NODE_CONTROL_TOKEN_FILE` |
| 旧令牌 | `--control-previous-token` | `LM_NODE_CONTROL_PREVIOUS_TOKENS` |
| CORS 来源 | `--cors-allow-origin` | `LM_NODE_CORS_ALLOW_ORIGIN` |
| 同步 peer | `--sync-peer` | - |
| 同步 peer token | `--sync-peer-token` | `LM_NODE_SYNC_PEER_TOKEN` |
| 同步 peer token 文件 | `--sync-peer-token-file` | `LM_NODE_SYNC_PEER_TOKEN_FILE` |
| 同步周期 | `--sync-interval-seconds` | - |
| 同步退避上限 | `--sync-max-backoff-seconds` | - |
| DHT 复制系数 | `--dht-replication-factor` | `LM_NODE_DHT_REPLICATION_FACTOR` |
| DHT 路由返回数 | `--dht-routing-refresh-limit` | `LM_NODE_DHT_ROUTING_REFRESH_LIMIT` |
| DHT 路由目标数 | `--dht-routing-refresh-max-targets` | `LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS` |
| DHT peer 隔离阈值 | `--dht-peer-quarantine-consecutive-failures` | `LM_NODE_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES` |
| 限流窗口 | `--rate-limit-window-seconds` | - |
| 限流请求数 | `--rate-limit-max-requests` | - |
| 日志格式 | `--log-format` | `LM_NODE_LOG_FORMAT` |
| Mailbox 总字节 | `--max-mailbox-bytes` | `LM_NODE_MAX_MAILBOX_BYTES` |
| 单用户字节 | `--max-mailbox-bytes-per-user` | `LM_NODE_MAX_MAILBOX_BYTES_PER_USER` |
| 单用户消息数 | `--max-mailbox-messages-per-user` | `LM_NODE_MAX_MAILBOX_MESSAGES_PER_USER` |

## node-admin 运维面板

`apps/node-admin` 是独立 Vue 前端，用于运维者查看和操作节点：

- 健康状态；
- 运行统计；
- sync peer；
- DHT 维护、复制、路由刷新、查找；
- snapshot 导入/导出。

它只调用节点 HTTP 控制面，不加载用户身份，也不使用 WASM。需要用户身份签名的操作仍在聊天 App 内完成。

跨域访问节点时，需要同时配置：

1. 控制面 token：`--control-token` 或 `--control-token-file`。
2. CORS 白名单：`--cors-allow-origin <node-admin-origin>`。

示例：

```bash
lm_node serve-control \
  --bind 0.0.0.0:8787 \
  --control-token "$(cat /etc/lm-node/control.token)" \
  --cors-allow-origin http://127.0.0.1:4174
```


### 本地 `/admin/` 管理页

如果你想把 node-admin 直接挂到 `lm_node` 上，请看 `docs/overview/DEV_WORKFLOW.md` 的“本机节点 + 内嵌 `/admin/`”小节。那里说明了：

- `./scripts/dev-run.sh node --local` 会自动检查或构建 `apps/node-admin/dist`；
- 需要时会用 `NODE_ADMIN_BASE=/admin/` 重新构建前端；
- 静态文件最终挂到 `http://127.0.0.1:8787/admin/`；
- `--web-admin` 仅适合作为本机 loopback 运维入口。

如果 node-admin 使用 HTTPS，而节点仍是 HTTP，浏览器会阻止混合内容。此时应为节点配置 TLS 反向代理，或在本机用 HTTP 方式打开 node-admin。

## 安全部署建议

- 公网只暴露反向代理端口，不直接暴露未保护的 `8787`。
- 控制面必须配置 token。
- token 文件权限建议 `0600`。
- `state_db` 所在磁盘或目录应由系统层加密保护。
- 反向代理应启用 HTTPS。
- CORS 白名单应只包含实际 Web / admin 来源。
- 定期查看 `/control/stats` 和 `/control/metrics`。
