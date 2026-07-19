# LM Node 配置文件

## 快速上手

```bash
# 1. 本机试用（全默认，监听 127.0.0.1:8787，仅本机可访问）
lm_node serve-control

# 2. 对外提供服务（绑定所有网卡 + Bearer token + SQLite 持久化）
lm_node serve-control --bind 0.0.0.0:8787 --control-token "$(cat /etc/lm-node/control.token)" --state-db /var/lib/lm-node/state.sqlite3

# 3. 生产推荐：把所有选项写进 JSON 配置文件，命令行只需一个参数
lm_node serve-control --config-file node.json
```

`serve-control` 有约 19 个选项，但绝大多数有合理默认值——日常只需 `--bind` / `--control-token` / `--state-db` 三个。完整选项见 `lm_node help` 或下方字段表。

## 配置文件

`lm_node serve-control` 支持 JSON 配置文件：

```bash
lm_node serve-control --config-file node.json
# 或
LM_NODE_CONFIG_FILE=node.json lm_node serve-control
```

优先级：

```text
CLI 参数 > 环境变量 > config file > 默认值
```

敏感值建议使用环境变量或 `*_file` 字段，不建议直接写进配置文件。

## 示例

见 `docs/examples/lm-node.config.example.json`。

## 字段

| 字段 | 类型 | 默认值 | 说明 |
|---|---:|---:|---|
| `bind` | string | `127.0.0.1:8787` | 控制面监听地址。生产部署建议绑定 loopback，由反向代理负责 TLS。 |
| `peer_id` | string | `lm-node-dev` | 本节点 public peer id。 |
| `state_db` | string | 无 | 明文 SQLite 正式状态数据库；按表保存 mailbox、prekey bundle、signed one-time-prekey records、consumed prekey、public peer/routing peer、DHT record 等节点状态。state_db 存的是中继/邮箱运营状态（离线消息已是端到端密文，prekey/public-peer/DHT 记录本就公开），磁盘静态保护由整盘加密（LUKS/dm-crypt）承担。 |
| `state_file` | string | 无 | 兼容 JSON snapshot 状态文件；保存时采用同目录临时文件 + fsync + rename；Unix 下保存后权限收紧为 `0600`。可与 `state_db` 同时配置作为调试导出。 |
| `control_token` | string | 无 | 控制面 Bearer token。配置后除 `/health` 外都要求 `Authorization: Bearer ...`。 |
| `control_token_file` | string | 无 | 从文件读取控制面 Bearer token；文件内容会 trim，空文件报错；Unix 下要求 regular file 且权限不能向 group/other 开放（建议 `chmod 600`），并拒绝 symlink。 |
| `control_previous_tokens` | string[] | `[]` | 旧控制面 Bearer token 列表，用于无停机轮换 grace window；只应短期保留。 |
| `cors_allow_origins` | string[] | `[]` | CORS Origin 白名单；空列表时仍只由 token/loopback 保护控制面。 |
| `sync_interval_seconds` | integer | `0` | 自动 sync 周期；`0` 表示关闭自动 sync 与同步后的 DHT runner。 |
| `sync_max_backoff_seconds` | integer | `300` | snapshot sync 失败指数退避上限。 |
| `sync_peers` | object[] | `[]` | 自动 sync/control-peer DHT runner 使用的 control peers。 |
| `dht_replication_factor` | integer | `3` | 同步周期后 DHT `StoreRecord` runner 的 replication factor；`0` 可关闭 replication runner。 |
| `dht_routing_refresh_limit` | integer | `8` | 每次 `FindNode` 请求希望返回的节点数，运行时会 clamp 到 `1..=64`。 |
| `dht_routing_refresh_max_targets` | integer | `8` | 每轮最多查询多少个 routing refresh target；`0` 可关闭 refresh runner。 |
| `dht_peer_quarantine_consecutive_failures` | integer | `5` | DHT runner 跳过处于退避期的连续失败 peer 的阈值；`0` 可关闭 quarantine。 |
| `rate_limit_window_seconds` | integer | `60` | per-client IP 基础限流窗口；`0` 表示关闭限流。 |
| `rate_limit_max_requests` | integer | `600` | 每窗口最大请求数；`0` 表示关闭限流。 |
| `log_format` | string | `text` | 控制面 stdout 日志格式；可选 `text` 或 `json`。`json` 会输出单行 JSON，字段包含 `ts`、`level`、`event`、`message`、`fields`。 |
| `mailbox_global_rate_limit_window_seconds` | integer/null | `null` | Mailbox `push` 全局限流窗口；与 `mailbox_global_rate_limit_max_messages` 同时配置且均大于 0 才启用。 |
| `mailbox_global_rate_limit_max_messages` | integer/null | `null` | 节点在窗口内最多可成功保存的 MailboxMessage 总数；超限返回 `429`。 |
| `max_mailbox_bytes` | integer/null | `10485760` | 节点最多保留的未 ACK MailboxMessage 总估算字节数；超限返回 `400` / `PayloadTooLarge`。 |
| `max_mailbox_bytes_per_user` | integer/null | `2097152` | 每个收件用户最多保留的未 ACK MailboxMessage 估算字节数；超限返回 `400` / `PayloadTooLarge`。 |
| `max_mailbox_messages_per_user` | integer/null | `1000` | 每个收件用户最多保留的未 ACK MailboxMessage 数；超限返回 `400` / `PayloadTooLarge`。 |
| `mailbox_sender_rate_limit_window_seconds` | integer/null | `null` | Mailbox `push` 按发送者 UserID 的限流窗口；与 `mailbox_sender_rate_limit_max_messages` 同时配置且均大于 0 才启用。 |
| `mailbox_sender_rate_limit_max_messages` | integer/null | `null` | 每个发送者在窗口内最多可 `push` 的 MailboxMessage 数；超限返回 `429`。 |

### `sync_peers[]`

| 字段 | 类型 | 说明 |
|---|---:|---|
| `url` | string | 远端 control plane URL，目前仅支持 `http://host:port`。 |
| `peer_id` | string | 可选；远端 public peer id。配置后，DHT StoreRecord replication 会按 closest-k plan 只发送给匹配目标 peer；未配置任何 `peer_id` 时保持旧行为：发送给所有 control peers。 |
| `token` | string | 远端控制面 Bearer token。 |
| `token_file` | string | 从文件读取远端 Bearer token；Unix 下同样要求 secret 文件权限收紧（建议 `chmod 600`）。 |

## 对应 CLI / 环境变量

常用字段可由 CLI 或环境变量覆盖：

| 配置字段 | CLI | 环境变量 |
|---|---|---|
| config file | `--config-file` | `LM_NODE_CONFIG_FILE` |
| `bind` | `--bind` | - |
| `peer_id` | `--peer-id` | - |
| `state_db` | `--state-db` | - |
| `state_file` | `--state-file` | - |
| `control_token` | `--control-token` | `LM_NODE_CONTROL_TOKEN` |
| `control_token_file` | `--control-token-file` | `LM_NODE_CONTROL_TOKEN_FILE` |
| `control_previous_tokens` | `--control-previous-token old1,old2` | `LM_NODE_CONTROL_PREVIOUS_TOKENS` |
| `cors_allow_origins` | `--cors-allow-origin` | `LM_NODE_CORS_ALLOW_ORIGIN` |
| `sync_peers[].url` | `--sync-peer` | - |
| sync peer shared token | `--sync-peer-token` | `LM_NODE_SYNC_PEER_TOKEN` |
| sync peer shared token file | `--sync-peer-token-file` | `LM_NODE_SYNC_PEER_TOKEN_FILE` |
| `sync_interval_seconds` | `--sync-interval-seconds` | - |
| `sync_max_backoff_seconds` | `--sync-max-backoff-seconds` | - |
| `dht_replication_factor` | `--dht-replication-factor` | `LM_NODE_DHT_REPLICATION_FACTOR` |
| `dht_routing_refresh_limit` | `--dht-routing-refresh-limit` | `LM_NODE_DHT_ROUTING_REFRESH_LIMIT` |
| `dht_routing_refresh_max_targets` | `--dht-routing-refresh-max-targets` | `LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS` |
| `dht_peer_quarantine_consecutive_failures` | `--dht-peer-quarantine-consecutive-failures` | `LM_NODE_DHT_PEER_QUARANTINE_CONSECUTIVE_FAILURES` |
| `rate_limit_window_seconds` | `--rate-limit-window-seconds` | - |
| `rate_limit_max_requests` | `--rate-limit-max-requests` | - |
| `log_format` | `--log-format` | `LM_NODE_LOG_FORMAT` |
| `mailbox_global_rate_limit_window_seconds` | `--mailbox-global-rate-limit-window-seconds` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_WINDOW_SECONDS` |
| `mailbox_global_rate_limit_max_messages` | `--mailbox-global-rate-limit-max-messages` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_MAX_MESSAGES` |
| `max_mailbox_bytes` | `--max-mailbox-bytes` | `LM_NODE_MAX_MAILBOX_BYTES` |
| `max_mailbox_bytes_per_user` | `--max-mailbox-bytes-per-user` | `LM_NODE_MAX_MAILBOX_BYTES_PER_USER` |
| `max_mailbox_messages_per_user` | `--max-mailbox-messages-per-user` | `LM_NODE_MAX_MAILBOX_MESSAGES_PER_USER` |
| `mailbox_sender_rate_limit_window_seconds` | `--mailbox-sender-rate-limit-window-seconds` | `LM_NODE_MAILBOX_SENDER_RATE_LIMIT_WINDOW_SECONDS` |
| `mailbox_sender_rate_limit_max_messages` | `--mailbox-sender-rate-limit-max-messages` | `LM_NODE_MAILBOX_SENDER_RATE_LIMIT_MAX_MESSAGES` |

## node-admin 远程运维面板

`apps/node-admin` 是一个独立的 Vue 前端，用于运维者监控和操作 lm_node 实例（健康、运行统计、sync peer、DHT 维护/复制/路由/查找、快照导入导出）。它只调用节点的 HTTP 控制面 REST API，**不加载任何用户身份、不使用 WASM**；需要身份签名的操作（PublicPeer / PreKey 发布）仍在聊天 App。

远程（跨域）访问节点时，必须让节点同时具备：

- **控制面令牌**：`--control-token <token>`（或 `--control-token-file`）。未配置 token 时节点仅允许 loopback 客户端访问，跨域请求会被拒绝。
- **CORS 白名单**：`--cors-allow-origin <admin-origin>`，值为 node-admin 页面的 Origin（如 `http://127.0.0.1:4174`）。

示例：

```bash
lm_node serve-control \
  --bind 0.0.0.0:8787 \
  --control-token "$(cat /etc/lm-node/control.token)" \
  --cors-allow-origin http://127.0.0.1:4174
```

在 node-admin 页面填入节点地址和上面的 token 即可连接。令牌只保存在运维者本机浏览器的 localStorage。

**混合内容提醒**：若 node-admin 以 https 提供（如 GitHub Pages），而节点是明文 http，浏览器会阻断请求。此时应在本机以 http 提供 node-admin，或为节点配置 TLS/反向代理。
