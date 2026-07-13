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
| `state_file` | string | 无 | 兼容 JSON snapshot 状态文件；保存时采用临时文件 + fsync + rename。可与 `state_db` 同时配置作为调试导出。 |
| `control_token` | string | 无 | 控制面 Bearer token。配置后除 `/health` 外都要求 `Authorization: Bearer ...`。 |
| `control_token_file` | string | 无 | 从文件读取控制面 Bearer token；文件内容会 trim，空文件报错。 |
| `cors_allow_origins` | string[] | `[]` | CORS Origin 白名单；空列表时仍只由 token/loopback 保护控制面。 |
| `sync_interval_seconds` | integer | `0` | 自动 sync 周期；`0` 表示关闭自动 sync 与同步后的 DHT runner。 |
| `sync_max_backoff_seconds` | integer | `300` | snapshot sync 失败指数退避上限。 |
| `sync_peers` | object[] | `[]` | 自动 sync/control-peer DHT runner 使用的 control peers。 |
| `dht_replication_factor` | integer | `3` | 同步周期后 DHT `StoreRecord` runner 的 replication factor；`0` 可关闭 replication runner。 |
| `dht_routing_refresh_limit` | integer | `8` | 每次 `FindNode` 请求希望返回的节点数，运行时会 clamp 到 `1..=64`。 |
| `dht_routing_refresh_max_targets` | integer | `8` | 每轮最多查询多少个 routing refresh target；`0` 可关闭 refresh runner。 |
| `rate_limit_window_seconds` | integer | `60` | per-client IP 基础限流窗口；`0` 表示关闭限流。 |
| `rate_limit_max_requests` | integer | `600` | 每窗口最大请求数；`0` 表示关闭限流。 |
| `log_format` | string | `text` | 控制面 stdout 日志格式；可选 `text` 或 `json`。`json` 会输出单行 JSON，字段包含 `ts`、`level`、`event`、`message`、`fields`。 |
| `mailbox_global_rate_limit_window_seconds` | integer/null | `null` | Mailbox `push` 全局限流窗口；与 `mailbox_global_rate_limit_max_messages` 同时配置且均大于 0 才启用。 |
| `mailbox_global_rate_limit_max_messages` | integer/null | `null` | 节点在窗口内最多可成功保存的 MailboxMessage 总数；超限返回 `429`。 |
| `mailbox_sender_rate_limit_window_seconds` | integer/null | `null` | Mailbox `push` 按发送者 UserID 的限流窗口；与 `mailbox_sender_rate_limit_max_messages` 同时配置且均大于 0 才启用。 |
| `mailbox_sender_rate_limit_max_messages` | integer/null | `null` | 每个发送者在窗口内最多可 `push` 的 MailboxMessage 数；超限返回 `429`。 |

### `sync_peers[]`

| 字段 | 类型 | 说明 |
|---|---:|---|
| `url` | string | 远端 control plane URL，目前仅支持 `http://host:port`。 |
| `peer_id` | string | 可选；远端 public peer id。配置后，DHT StoreRecord replication 会按 closest-k plan 只发送给匹配目标 peer；未配置任何 `peer_id` 时保持旧行为：发送给所有 control peers。 |
| `token` | string | 远端 control plane Bearer token。 |
| `token_file` | string | 从文件读取远端 Bearer token。 |

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
| `cors_allow_origins` | `--cors-allow-origin` | `LM_NODE_CORS_ALLOW_ORIGIN` |
| `sync_peers[].url` | `--sync-peer` | - |
| sync peer shared token | `--sync-peer-token` | `LM_NODE_SYNC_PEER_TOKEN` |
| sync peer shared token file | `--sync-peer-token-file` | `LM_NODE_SYNC_PEER_TOKEN_FILE` |
| `sync_interval_seconds` | `--sync-interval-seconds` | - |
| `sync_max_backoff_seconds` | `--sync-max-backoff-seconds` | - |
| `dht_replication_factor` | `--dht-replication-factor` | `LM_NODE_DHT_REPLICATION_FACTOR` |
| `dht_routing_refresh_limit` | `--dht-routing-refresh-limit` | `LM_NODE_DHT_ROUTING_REFRESH_LIMIT` |
| `dht_routing_refresh_max_targets` | `--dht-routing-refresh-max-targets` | `LM_NODE_DHT_ROUTING_REFRESH_MAX_TARGETS` |
| `rate_limit_window_seconds` | `--rate-limit-window-seconds` | `LM_NODE_RATE_LIMIT_WINDOW_SECONDS` |
| `rate_limit_max_requests` | `--rate-limit-max-requests` | `LM_NODE_RATE_LIMIT_MAX_REQUESTS` |
| `log_format` | `--log-format` | `LM_NODE_LOG_FORMAT` |
| `mailbox_global_rate_limit_window_seconds` | `--mailbox-global-rate-limit-window-seconds` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_WINDOW_SECONDS` |
| `mailbox_global_rate_limit_max_messages` | `--mailbox-global-rate-limit-max-messages` | `LM_NODE_MAILBOX_GLOBAL_RATE_LIMIT_MAX_MESSAGES` |
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
- 面向不完全可信客户端开放 Mailbox 时，建议同时启用控制面 per-client IP 限流、mailbox 全局限流和 mailbox sender 限流，例如 `rate_limit_*` + `mailbox_global_rate_limit_*` + `mailbox_sender_rate_limit_*`。
- Mailbox `push` 拒绝原因会累计到 `maintenance.mailbox_push_rejects`，并通过 `/control/metrics` 的 `lm_node_mailbox_push_rejections_total{reason=...}` 暴露，便于观察异常 payload、重复消息和限流命中。
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
- 确认已配置 `control_token_file`，且 secret 文件权限只允许节点进程读取。
- 确认反向代理证书自动续期正常。
- 确认 `cors_allow_origins` 只包含实际使用的 Web 来源；如果不是浏览器客户端，可保持空列表并依赖 Bearer token。
- 确认反向代理和节点请求体上限不超过控制面读取上限；当前 `lm_node` 单请求 body 上限为 4 MiB。
- 确认 `/control/stats`、`/control/metrics` 同样受 Bearer token 保护，不要单独公开。

## Token 轮换策略

当前 `lm_node` 每个控制面实例只接受一个 `control_token`。推荐使用 `control_token_file`，通过原子替换 secret 文件并重启节点完成轮换。

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

5. 确认 `/control/stats` 中 `unauthorized` 没有持续增长，再删除旧 token 的客户端配置。

注意事项：

- 不建议把 token 写进 shell history；优先使用 secret 文件。
- 如果有多个互相 sync 的节点，应逐个滚动更新：先让所有 peer 配置接受/使用新 token，再重启被访问节点。
- 当前没有多 token grace window；需要无中断轮换时，应在反向代理层临时接受旧 token 并改写为新 token，或部署第二个节点 URL 后切流。

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

当前正式持久化路径是 SQLite `state_db`；JSON `state_file` 仅作为兼容 snapshot 导出路径。建议备份：

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
