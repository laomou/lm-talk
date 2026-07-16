# LM Node 配置文件

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
| `state_db` | string | 无 | SQLite 正式状态数据库；按表保存 mailbox、prekey bundle、signed one-time-prekey records、consumed prekey、public peer/routing peer、DHT record 等节点状态。 |
| `state_db_require_encryption` | bool | `false` | 要求数据库级加密时 fail-closed；当前构建仍是 plain SQLite，设为 `true` 会拒绝启动，避免误以为已启用 SQLCipher。 |
| `state_file` | string | 无 | 兼容 JSON snapshot 状态文件；保存时采用同目录临时文件 + fsync + rename；Unix 下保存后权限收紧为 `0600`。设置 `LM_NODE_STATE_FILE_PASSPHRASE` 后会以应用层加密格式保存/读取。可与 `state_db` 同时配置作为调试导出。 |
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
| `token` | string | 远端 control plane Bearer token。 |
| `token_file` | string | 从文件读取远端 Bearer token；Unix 下同样要求 secret 文件权限收紧（建议 `chmod 600`）。 |

## 对应 CLI / 环境变量

常用字段可由 CLI 或环境变量覆盖：

| 配置字段 | CLI | 环境变量 |
|---|---|---|
| config file | `--config-file` | `LM_NODE_CONFIG_FILE` |
| `bind` | `--bind` | - |
| `peer_id` | `--peer-id` | - |
| `state_db` | `--state-db` | - |
| `state_db_require_encryption` | `--state-db-require-encryption` | `LM_NODE_STATE_DB_REQUIRE_ENCRYPTION` |
| `state_file_passphrase` | - | `LM_NODE_STATE_FILE_PASSPHRASE` |
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
| `rate_limit_window_seconds` | `--rate-limit-window-seconds` | `LM_NODE_RATE_LIMIT_WINDOW_SECONDS` |
| `rate_limit_max_requests` | `--rate-limit-max-requests` | `LM_NODE_RATE_LIMIT_MAX_REQUESTS` |
| `log_format` | `--log-format` | `LM_NODE_LOG_FORMAT` |
| `mailbox_global_rate_limit_window_seconds` | `--mailbox-global-rate-limit-window-seconds` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_WINDOW_SECONDS` |
| `mailbox_global_rate_limit_max_messages` | `--mailbox-global-rate-limit-max-messages` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_MAX_MESSAGES` |
| `max_mailbox_bytes` | `--max-mailbox-bytes` | `LM_NODE_MAX_MAILBOX_BYTES` |
| `max_mailbox_bytes_per_user` | `--max-mailbox-bytes-per-user` | `LM_NODE_MAX_MAILBOX_BYTES_PER_USER` |
| `max_mailbox_messages_per_user` | `--max-mailbox-messages-per-user` | `LM_NODE_MAX_MAILBOX_MESSAGES_PER_USER` |
| `mailbox_sender_rate_limit_window_seconds` | `--mailbox-sender-rate-limit-window-seconds` | `LM_NODE_MAILBOX_SENDER_RATE_LIMIT_WINDOW_SECONDS` |
| `mailbox_sender_rate_limit_max_messages` | `--mailbox-sender-rate-limit-max-messages` | `LM_NODE_MAILBOX_SENDER_RATE_LIMIT_MAX_MESSAGES` |


## PreKey 存储与接口兼容

`/prekey/publish` 请求体：

```json
{
  "prekey_bundle_text": "lm-prekey-bundle-v1:...",
  "signed_one_time_prekey_record_texts": [
    "lm-signed-one-time-prekey-v1:..."
  ]
}
```

响应和 `/prekey/get` / `/prekey/status` 会包含：

- `remaining_one_time_prekeys` / `low_one_time_prekeys` / `replenishment_required`：客户端补货提示。
- `signed_one_time_prekey_records`：当前 bundle 对应的已发布 signed OTK record 数量。
- `/prekey/get?consume=true` 的 `selected_signed_one_time_prekey_record_text`：发起方应优先用它创建 X3DH initial message；为空时再回退 `selected_one_time_prekey_id`。

SQLite `state_db` 中对应表：

- `prekey_bundles`：每个 UserID 当前 signed prekey bundle。
- `signed_one_time_prekey_records`：独立 signed OTK records，主键为 `(user_id, signed_prekey_id, key_id)`。
- `consumed_one_time_prekeys`：已消费 key id，signed prekey 轮换时会重置。

节点不会生成用户密钥；低水位补货必须由客户端持有 private prekey bundle 后重新发布。

## 安全部署建议

- 生产环境必须配置 `control_token` 或 `control_token_file`；未配置 token 时非 `/health` API 仅允许 loopback 客户端。
- Bearer token、同步 peer token 建议使用 secret 文件或环境变量注入。
- 当前控制面内置 HTTP，不直接提供 TLS；公网部署应放在 TLS 反向代理后。
- 面向不完全可信客户端开放 Mailbox 时，建议同时启用控制面 per-client IP 限流、mailbox 全局限流、mailbox sender 限流、节点/收件人存储配额，例如 `rate_limit_*` + `mailbox_global_rate_limit_*` + `mailbox_sender_rate_limit_*` + `max_mailbox_bytes` + `max_mailbox_bytes_per_user` + `max_mailbox_messages_per_user`。
- Mailbox `push` 拒绝原因会累计到 `maintenance.mailbox_push_rejects`，并通过 `/control/metrics` 的 `lm_node_mailbox_push_rejections_total{reason=...}` 暴露，便于观察异常 payload、重复消息和限流命中。
- Mailbox `ack` 拒绝原因会累计到 `maintenance.mailbox_ack_rejects`，并通过 `/control/metrics` 的 `lm_node_mailbox_ack_rejections_total{reason=...}` 暴露，便于观察异常 ACK 批次、无效 user_id 或超长 delivery id；`/health` 会暴露 Mailbox take/ack 上限、Mailbox 总字节/每用户字节/每用户消息数配额、控制面入站/peer 超时、control-peer 响应上限和 libp2p DHT RPC request/response/concurrency 上限和 libp2p DHT 连接数上限，便于确认反滥用参数。
- `GET /mailbox/status?user_id=...&delivery_id=...` 返回该用户邮箱的 `summary`（包含消息数、未取/已取未 ACK 数和当前 bytes 用量）、`max_bytes_per_user` 和可选 delivery 状态：`pending`（尚未被 take）、`delivered_unacked`（已被 take 但未 ack）、`acked`（节点已收到 ACK，并在短期 tombstone 内保留）、`absent_or_expired`（不存在或 tombstone/消息已过期清理）。该端点用于客户端区分“对方未取”“已取但 ACK 未完成”和“ACK 已完成”，也可用于展示用户 mailbox 配额压力。`/mailbox/push`、`/mailbox/take` 和 `/mailbox/ack` 响应也会返回当前 `pending_bytes` 与 `max_bytes_per_user`。
- `POST /sync/peer/reset` 请求体 `{ "url": "http://peer" }` 可清除指定 sync/DHT peer 的当前 `consecutive_failures`、`last_error` 和 `next_attempt_at`，保留累计 attempts/failures 作为历史，用于误隔离或网络恢复后手动解除退避。
- 控制面 HTTP parser 会拒绝超大 method/path/header line、冲突的重复 `Content-Length` 和非空 `Transfer-Encoding`，超大 body/header 分别返回 `413`/`431`，降低请求走私和解析歧义风险。
- 控制面所有 HTTP 响应都会附加浏览器安全响应头：`Cache-Control: no-store`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`、受限 `Permissions-Policy` 和 API 友好的 `Content-Security-Policy`，降低控制面状态、snapshot 或错误响应被浏览器缓存/嗅探/误用的风险。
- DHT record 拒绝原因会累计到 `maintenance.dht_record_rejects`，并通过 `/control/metrics` 的 `lm_node_dht_record_rejections_total{reason=...}` 暴露；统计覆盖控制面 store、DHT RPC StoreRecord、sync snapshot 导入以及 FindValue 返回的 found/closer records，便于观察垃圾记录、TTL 超限、value 超限或 key-kind-value 不匹配。Routing peer / closer node 拒绝原因会累计到 `maintenance.routing_peer_rejects`，并通过 `lm_node_routing_peer_rejections_total{reason=...}` 暴露，便于观察无效签名、缺少 identity public key、node_id 不匹配、本机节点、地址数量超限或单条地址过长等异常路由数据；节点会拒绝超过 16 个地址或单条地址超过 512 bytes 的 routing peer，避免恶意 announce 膨胀 routing table、snapshot 和控制面响应。HTTP/libp2p DHT transport 还会校验响应体里的 logical `request_id` 必须与请求一致，并把 DHT `Error` 响应作为失败处理，避免异常 peer 的串扰响应被误用；FindValue / FindNode 客户端侧只处理有界数量的返回 records/nodes，即使远端忽略请求 limit，也不会把超量 closer records/nodes 全部合并；routing refresh 的 FindNode 响应还会过滤自引用/重复节点，并在响应 peer id 已知时拒绝未比响应 peer 更接近 refresh target 的节点；libp2p DHT RPC codec 显式限制 request/response 大小并将并发 streams 收紧到项目上限；libp2p swarm 还启用 connection-limits，限制 pending incoming/outgoing、established incoming/outgoing、total 和 per-peer 连接数，降低异常 peer 的内存、并发和连接耗尽风险。
- `GET /dht/key?kind=public-peer|prekey|mailbox-hint&value=...` 可派生 DHT record key，便于把 peer_id/UserID 转成 `FindValue` 所需 64 位 hex key；`GET /dht/maintenance?factor=N&limit=N&max_targets=N` 可一键运行 replication 与 routing refresh；`GET /dht/replicate?factor=N` 可手动触发 due DHT record replication；`GET /dht/routing-refresh?limit=N&max_targets=N` 可手动触发 routing refresh，立即使用已配置 sync/control peers 与当前 DHT transport 刷新 routing table；`GET /dht/find-value?key=<hex>&limit=N&max_peers=N&alpha=N`（也支持 `kind=public-peer|prekey|mailbox-hint&value=...` 直接派生 key）会使用已配置 sync/control peers 与当前 DHT transport 执行有界迭代 FindValue 查询，即使 `sync_interval_seconds=0` 关闭自动 snapshot sync 也可手动查询；HTTP-control transport 会把 routing table 中同 scheme `http://` routing peers 加入候选，但不会向发现的第三方 peer 传播已配置 bearer token；每轮会按 `alpha` 并发查询候选 peer；当响应 peer 的 peer_id 已知时，会拒绝未比响应 peer 更接近目标 key 的 closer nodes，并在指标中记录 `rejected_non_closer`；还会拒绝 closer nodes 中的自引用、已查询或已入队重复候选，并在指标中记录 `rejected_duplicate`；响应返回命中的 record（如有）以及 attempts/successes/found/closer/query_rounds/alpha/exhausted/peers_quarantined 统计；`/control/stats` 和 `/control/metrics` 会聚合手动 FindValue runs、attempts、命中记录、closer records/nodes、quarantine 和 exhausted 次数；DHT runner 会优先尝试 `sync_status` 中 consecutive_failures/failures 更低的健康配置 peer；replication、routing refresh 和 FindValue 的 DHT RPC 成功/失败会回写 peer health、退避和连续失败状态；当 FindNode/FindValue 响应只提供不前进或重复的 routing hints 时，也会把响应 peer 记为可疑失败进入退避链路；并在退避期内跳过 consecutive_failures 达到配置阈值（默认 5）的本轮 quarantine peer，next_attempt_at 到期后会重新纳入候选；`/control/metrics` 暴露 `lm_node_dht_peer_quarantined{peer=...}`，并聚合 replication/refresh/find-value 中被 quarantine 跳过的 peer 数，便于告警；该端点仍是 scaffold，生产查询仍需更完整 peer scoring 和更强恶意路由防护。
- PreKey 发布建议使用新版客户端生成的 `signed_one_time_prekey_record_texts[]`。节点会优先按独立 signed one-time-prekey records 选择和消费 OTK；旧 bundle 内 `one_time_prekeys[]` 仍作为兼容回退。
- 生产环境建议设置 `log_format = "json"` 或 `--log-format json`，便于 systemd/journald、容器平台或日志采集器按 `event` 和 `fields` 建索引。
- 当前 control-peer DHT routing refresh 合并属于已配置 control peer 信任边界内的 bootstrap 能力；`RoutingPeer` 已可携带并持久化 identity public key，节点在 verified merge 路径会校验 announce 签名。开放传输层 DHT 还需要真正的网络 RPC 与端到端策略。

## 结构化日志

默认 `log_format = "text"` 会输出兼容旧版本的人类可读文本。设置 `log_format = "json"` 后，`serve-control` 的启动、请求访问、snapshot sync、DHT runner、状态保存错误等日志会以单行 JSON 输出：

```json
{"ts":1720000000,"level":"info","event":"control.request","message":"control request: GET /health status=200 duration_micros=42","fields":{"method":"GET","path":"/health","endpoint":"GET /health","status":200,"duration_micros":42,"request_body_bytes":0,"response_body_bytes":128,"remote_addr":"127.0.0.1:54321"}}
```

常见 `event`：

- `control.start` / `control.security` / `control.rate_limit`
- `control.request`
- `sync.snapshot.success` / `sync.snapshot.error`
- `dht.replication.run` / `dht.replication.error`
- `dht.routing_refresh.run` / `dht.routing_refresh.error`
- `state_file.save_error` / `state_db.save_error`

## TLS / 反向代理部署

`lm_node serve-control` 当前只提供内置 HTTP 控制面。公网或跨不可信网络部署时，推荐：

1. `lm_node` 绑定本机 loopback，例如 `127.0.0.1:8787`。
2. 配置 `control_token_file`，所有非 `/health` 请求必须带 Bearer token。
3. 由 Nginx、Caddy、Traefik 等反向代理终止 TLS。
4. 反向代理只转发必要路径到本机 `lm_node`。
5. Web/PWA 或其他客户端使用 `https://sync.example|<token>` 作为同步服务地址。

### 推荐 `lm_node` 配置

```json
{
  "bind": "127.0.0.1:8787",
  "peer_id": "lm-node-prod-1",
  "state_db": "/var/lib/lm-node/state.sqlite3",
  "control_token_file": "/etc/lm-node/control.token",
  "cors_allow_origins": ["https://chat.example"],
  "log_format": "json",
  "rate_limit_window_seconds": 60,
  "rate_limit_max_requests": 600,
  "mailbox_global_rate_limit_window_seconds": 60,
  "mailbox_global_rate_limit_max_messages": 1000,
  "max_mailbox_bytes": 10485760,
  "max_mailbox_bytes_per_user": 2097152,
  "max_mailbox_messages_per_user": 1000,
  "mailbox_sender_rate_limit_window_seconds": 60,
  "mailbox_sender_rate_limit_max_messages": 120
}
```

启动：

```bash
lm_node serve-control --config-file /etc/lm-node/node.json
```

### Nginx 示例

```nginx
server {
    listen 443 ssl http2;
    server_name sync.example;

    ssl_certificate /etc/letsencrypt/live/sync.example/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sync.example/privkey.pem;

    client_max_body_size 4m;

    location / {
        proxy_pass http://127.0.0.1:8787;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### Caddy 示例

```caddyfile
sync.example {
    encode zstd gzip
    request_body {
        max_size 4MB
    }
    reverse_proxy 127.0.0.1:8787
}
```

### 部署检查清单

- 确认 `lm_node` 未直接监听公网地址：优先使用 `bind = "127.0.0.1:8787"`。
- 确认已配置 `control_token_file`，且 secret 文件权限只允许节点进程读取（Unix 下 `lm_node` 会拒绝 group/other 可读写执行或 symlink secret 文件，建议 `chmod 600`）。
- 确认反向代理证书自动续期正常。
- 确认 `cors_allow_origins` 只包含实际使用的 Web 来源；如果不是浏览器客户端，可保持空列表并依赖 Bearer token。
- 确认反向代理和节点请求体上限不超过控制面读取上限；当前 `lm_node` 单请求 body 上限为 4 MiB。
- 确认 `/control/stats`、`/control/metrics` 同样受 Bearer token 保护，不要单独公开。

## Token 轮换策略

`lm_node` 支持一个当前 `control_token` 加一个短期 `control_previous_tokens[]` grace window。推荐使用 `control_token_file` 保存当前 token，并在轮换期间通过配置、CLI 或环境变量提供旧 token 列表；所有旧 token 都应在客户端切换完成后尽快移除。

推荐流程：

1. 生成新 token：

   ```bash
   install -d -m 700 /etc/lm-node
   openssl rand -hex 32 > /etc/lm-node/control.token.next
   chmod 600 /etc/lm-node/control.token.next
   ```

2. 先把新 token 分发给受信任客户端或同步 peer 配置，但保留旧 token 可用。
3. 在维护窗口内原子替换 token 文件并重启节点：

   ```bash
   mv /etc/lm-node/control.token.next /etc/lm-node/control.token
   systemctl restart lm-node
   ```

4. 用新 token 验证非 health API：

   ```bash
   curl -H "Authorization: Bearer $(cat /etc/lm-node/control.token)" \
     https://sync.example/sync/status
   ```

5. 更新所有客户端/同步 peer 使用新 token。确认 `/control/stats` 中 `unauthorized` 没有持续增长后，移除 `control_previous_tokens` 并再次重启节点。

注意事项：

- 不建议把 token 写进 shell history；优先使用 secret 文件。
- 如果有多个互相 sync 的节点，应逐个滚动更新：先在被访问节点加入 previous token，再让所有 peer 配置切换到新 token，最后移除 previous token。
- `control_previous_tokens` 是临时兼容窗口，不应长期保留；日志和指标不会输出 token 原文。

## systemd 部署示例

目录建议：

```bash
sudo install -d -o lm-node -g lm-node -m 750 /var/lib/lm-node
sudo install -d -o root -g lm-node -m 750 /etc/lm-node
sudo install -m 640 -o root -g lm-node node.json /etc/lm-node/node.json
sudo install -m 640 -o root -g lm-node control.token /etc/lm-node/control.token
sudo install -m 755 target/release/lm_node /usr/local/bin/lm_node
```

`/etc/systemd/system/lm-node.service`：

```ini
[Unit]
Description=LM Talk node control plane
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=lm-node
Group=lm-node
ExecStart=/usr/local/bin/lm_node serve-control --config-file /etc/lm-node/node.json
Restart=on-failure
RestartSec=5s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/lm-node
ReadOnlyPaths=/etc/lm-node

[Install]
WantedBy=multi-user.target
```

启用：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now lm-node
sudo systemctl status lm-node
```

## Container 部署示例

最小 Dockerfile：

```dockerfile
FROM debian:stable-slim
RUN useradd --system --create-home --home-dir /var/lib/lm-node lm-node
COPY target/release/lm_node /usr/local/bin/lm_node
USER lm-node
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/lm_node"]
CMD ["serve-control", "--config-file", "/etc/lm-node/node.json"]
```

运行示例：

```bash
docker run -d --name lm-node \
  --restart unless-stopped \
  -p 127.0.0.1:8787:8787 \
  -v /etc/lm-node:/etc/lm-node:ro \
  -v /var/lib/lm-node:/var/lib/lm-node \
  lm-node:local
```

容器内配置仍建议使用：

```json
{
  "bind": "0.0.0.0:8787",
  "state_db": "/var/lib/lm-node/state.sqlite3",
  "control_token_file": "/etc/lm-node/control.token"
}
```

宿主机只把端口发布到 `127.0.0.1`，再由宿主机反向代理提供 HTTPS。

## 数据备份 / 恢复

当前正式持久化路径是 SQLite `state_db`；连接会启用 WAL、`synchronous=FULL`、`busy_timeout=5000` 和 `foreign_keys=ON`，提升崩溃恢复和短暂锁竞争时的可靠性。Unix 下 `state_db` 主文件及 `-wal` / `-shm` sidecar 会在打开/保存后收紧为 `0600`，降低本机其他用户读取未加密状态的风险；`/health`、`/control/stats` 和 `/control/metrics` 会明确返回/暴露 `state_db_encrypted=false` / `lm_node_state_db_encrypted 0` 与 `state_db_permissions_hardened=true` / `lm_node_state_db_permissions_hardened 1`，避免把权限硬化误认为数据库级加密。若部署要求数据库级加密，可设置 `state_db_require_encryption=true` / `--state-db-require-encryption true` 让当前 plain SQLite 构建 fail-closed；该 fail-closed 同时覆盖 `serve-control` 和 `serve-dht-libp2p`。`state_file` 仍主要作为兼容 snapshot 导出路径；若设置 `LM_NODE_STATE_FILE_PASSPHRASE`，会保存为 `lm-node-state-file-v1:` 前缀的 XChaCha20-Poly1305 应用层加密文本，启动时读取该前缀文件也必须提供同一环境变量；未设置时保持纯 JSON 兼容格式。保存后同样收紧为 `0600`。建议备份：

- `state_db`，例如 `/var/lib/lm-node/state.sqlite3`
- 如启用了兼容导出，也备份 `state_file`，例如 `/var/lib/lm-node/state.json`
- 节点配置 `/etc/lm-node/node.json`
- secret 文件，例如 `/etc/lm-node/control.token`、sync peer token files
- 反向代理配置和证书自动续期配置

备份前可调用：

```bash
curl -H "Authorization: Bearer $(cat /etc/lm-node/control.token)" \
  https://sync.example/sync/snapshot > lm-node-snapshot.json
```

恢复方式：

1. 停止节点。
2. 恢复 `state_db`；如只有 JSON snapshot，可启动空库后通过 `POST /sync/import` 导入。
3. 恢复配置和 token 文件，确认权限。
4. 启动节点并检查 `/health`、`/sync/status`。
5. 检查 `/control/stats` / `/control/metrics`，确认 mailbox/DHT rejection 指标没有异常增长。

注意：

- snapshot 包含 mailbox、prekey bundle、signed one-time-prekey records、consumed prekey、public peer/routing peer 等节点状态，不包含用户身份私钥或 private prekey bundle。
- 如果 token 丢失，只能在服务器上重置 token 并让客户端更新配置。
- 恢复到旧 snapshot 可能重新暴露未 ack 的 mailbox delivery；客户端应依赖 message_id/delivery_id 去重。

## 升级兼容策略

升级前：

1. 备份 `state_db`，并可额外导出 `/sync/snapshot`。
2. 记录当前二进制版本或 Git commit。
3. 阅读 release notes / TODO 中的 snapshot schema 变化。

升级过程：

```bash
systemctl stop lm-node
install -m 755 target/release/lm_node /usr/local/bin/lm_node
systemctl start lm-node
```

回滚：

1. 停止节点。
2. 恢复旧二进制。
3. 如新版本写入了不兼容 state，应恢复升级前备份的 `state_db`。

兼容承诺：

- `NodeStateSnapshot.version` 用于标识 snapshot 格式。
- 新字段应使用 serde 默认值保持旧 snapshot 可读；当前 `signed_one_time_prekey_records` 字段缺失时会按旧 bundle 内 OTK 兼容处理。
- 删除或重命名字段前必须提供迁移路径。
