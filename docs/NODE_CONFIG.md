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
| `state_file` | string | 无 | JSON snapshot 状态文件；保存时采用临时文件 + fsync + rename。 |
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

### `sync_peers[]`

| 字段 | 类型 | 说明 |
|---|---:|---|
| `url` | string | 远端 control plane URL，目前仅支持 `http://host:port`。 |
| `token` | string | 远端 control plane Bearer token。 |
| `token_file` | string | 从文件读取远端 Bearer token。 |

## 对应 CLI / 环境变量

常用字段可由 CLI 或环境变量覆盖：

| 配置字段 | CLI | 环境变量 |
|---|---|---|
| config file | `--config-file` | `LM_NODE_CONFIG_FILE` |
| `bind` | `--bind` | - |
| `peer_id` | `--peer-id` | - |
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

## 安全部署建议

- 生产环境必须配置 `control_token` 或 `control_token_file`；未配置 token 时非 `/health` API 仅允许 loopback 客户端。
- Bearer token、同步 peer token 建议使用 secret 文件或环境变量注入。
- 当前控制面内置 HTTP，不直接提供 TLS；公网部署应放在 TLS 反向代理后。
- 当前 control-peer DHT routing refresh 合并属于已配置 control peer 信任边界内的 bootstrap 能力；开放传输层 DHT 还需要返回节点携带 identity public key 并做端到端签名校验。
