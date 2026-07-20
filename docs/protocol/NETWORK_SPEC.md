# 网络规格 v1

LM Talk 网络能力是可选的，采用 best-effort 语义。节点不接触消息或文件明文。

## 投递路径

| 路径 | 说明 |
| --- | --- |
| WebRTC DataChannel | 在线直连优先路径。 |
| Mailbox | 离线投递路径，载荷为端到端密文或签名控制对象。 |
| Outbox | 本地重试队列，使用指数退避。 |
| DHT | 发现 ContactCard、PreKey、MailboxHint、PublicPeer。 |
| Snapshot sync | 节点之间同步 Mailbox/DHT/PreKey 等运营状态。 |

Outbox 默认退避：30 秒、2 分钟、10 分钟、1 小时、6 小时，默认 7 天过期。

## 控制面鉴权

除 `/health` 外，节点控制面都需要安全边界：

- 未配置 control token：只允许 loopback 客户端访问。
- 配置 control token：必须携带 `Authorization: Bearer <token>`。

Web 同步服务输入格式：

```text
http://127.0.0.1:8787
http://192.168.1.23:8787|s3cr3t-token
http://[fd00::1234]:8787|s3cr3t-token
```

`/health` 免鉴权，因此 Web 还会探测需要鉴权的接口，以便明确提示 401。

## CORS

若使用 `--cors-allow-origin` 收紧来源，必须把 Web 或 node-admin 的 Origin 加入白名单。


## 控制面错误响应

新的节点控制面错误优先返回结构化 JSON：

```json
{
  "error_code": "UNAUTHORIZED",
  "message": "unauthorized",
  "recovery_hint": "请检查 Bearer token 或同步服务地址后的 |令牌。"
}
```

客户端应优先使用 `error_code` 分类，再回退到 HTTP 状态码和旧文本。常见错误码：

| 错误码 | HTTP | 含义 | 建议处理 |
| --- | --- | --- | --- |
| `UNAUTHORIZED` | 401 | token 缺失或错误 | 检查 `--control-token` 和地址后的 `|令牌`。 |
| `CORS_ORIGIN_NOT_ALLOWED` | 403 | Origin 不在白名单 | 更新 `--cors-allow-origin`。 |
| `ADMIN_LOOPBACK_ONLY` | 403 | `/admin/` 非本机访问 | 使用本机浏览器或 SSH 隧道。 |
| `CONTROL_RATE_LIMITED` | 429 | 控制面限流 | 稍后重试或调整限流配置。 |
| `MAILBOX_RATE_LIMITED` | 429 | Mailbox push 限流 | 稍后重试或调整 Mailbox 限流。 |
| `MAILBOX_QUOTA_EXCEEDED` | 413 | Mailbox 内容超过大小/配额 | 缩小内容、清理收件箱或调整配额。 |
| `DHT_RECORD_TOO_LARGE` | 413 | DHT record 超过大小限制 | 缩小 DHT record。 |
| `DHT_RECORD_EXPIRED` | 400 | DHT record 已过期 | 重新生成并发布。 |
| `DHT_RECORD_TTL_TOO_LONG` | 400 | DHT TTL 超限 | 降低 TTL。 |
| `API_NOT_FOUND` | 404 | API 路径不存在 | 检查节点版本和路径。 |
| `WEB_ADMIN_NOT_CONFIGURED` | 404 | 未配置 `--web-admin` | 指向 `node_admin.zip` 或 `apps/node-admin/dist`。 |
| `STATIC_FILE_NOT_FOUND` | 404 | `/admin/` 静态资源缺失 | 用 `NODE_ADMIN_BASE=/admin/` 重新构建。 |

## 跨设备部署

### 仅本机

```bash
lm_node serve-control
```

Web 填：

```text
http://127.0.0.1:8787
```

### 局域网

```bash
openssl rand -hex 16 > node.token
lm_node serve-control --bind 0.0.0.0:8787 --control-token-file node.token
```

其他设备填：

```text
http://<局域网IP>:8787|<令牌>
```

### 异地无公网 IP

可使用 Tailscale / ZeroTier，将节点地址填为虚拟网 IP。

### VPS / 公网

建议使用反向代理提供 HTTPS，并配置 control token 与 CORS 白名单。多个节点可通过 `sync_peers` 互相同步。

## 当前边界

节点网络可支撑 demo 和本地 federation 测试；当前目标不要求长期公网运行报告。
