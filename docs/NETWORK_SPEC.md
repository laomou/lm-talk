# LM Talk 网络规格 v1

网络能力是可选的，并采用 best-effort 语义。

- 在线直连优先使用 WebRTC DataChannel。
- 离线投递通过已配置的 `lm_node` 控制面使用 Mailbox。
- Outbox 重试使用指数退避：30s、2m、10m、1h、6h；默认过期时间为 7 天。
- Snapshot sync 可以从对端节点拉取 `/sync/snapshot` 并导入本地。
- DHT RPC 当前有 HTTP control-plane 和 libp2p request-response 两条实验性传输路径。

当前节点网络可支撑控制面同步、Mailbox、PreKey 和实验性 libp2p DHT RPC；它仍不是生产级 DHT/relay 网络。

## 控制面鉴权模型

节点对除 `GET /health` 外的所有接口做鉴权（`main.rs: request_is_authorized`）：

- **未配置 `--control-token`** → 仅允许 **loopback（127.0.0.1）** 客户端调用，其余来源一律 `401`。
- **配置了 `--control-token`** → 必须携带 `Authorization: Bearer <token>`（常量时间比较）。

网页端在「我 → 消息同步 → 同步服务」里，每行填 `URL` 或 `URL|令牌`：

```
http://127.0.0.1:8787
http://192.168.1.23:8787|s3cr3t-token
http://[fd00::1234]:8787|s3cr3t-token
```

> `/health` 免鉴权，会掩盖令牌问题；网页的"同步状态"会额外探测 `GET /sync/status`，鉴权失败时明确提示 `401`，不再误报"已连接"。

CORS：默认允许所有来源（返回 `*`）。若用 `--cors-allow-origin` 收紧，必须把网页来源（如 `http://192.168.1.23:5173`）列入，否则浏览器会拦截。

## 跨设备最小部署

### A. 仅本机
```
./scripts/run.sh node --local        # 绑定 127.0.0.1:8787，无需令牌
```
网页填 `http://127.0.0.1:8787`。

### B. 同一局域网、多设备
```
# 生成一个令牌
openssl rand -hex 16 > node.token
./scripts/run.sh node --lan --control-token-file node.token
```
其它设备网页填 `http://<局域网IP>:8787|<令牌>`。
（`--lan` 不带令牌时，别的设备只有 `/health` 能通，收发消息会 `401`。）

### C. 无公网 IP、异地 → Tailscale/ZeroTier
1. 各设备装 Tailscale，登录同一账号；
2. 任一台跑 `./scripts/run.sh node --lan --control-token-file node.token`；
3. 双方网页填 `http://<该设备的100.x.x.x>:8787|<令牌>`。

### D. 有公网可达地址 → VPS
在 VPS 上跑 B 的命令，双方填 `http://<vps-ip>:8787|<令牌>`。可用 `--sync-peer` 让多个节点互相拉取 `/sync/snapshot` 联邦同步。

> 以上均为文字/离线消息路径（mailbox）。音视频/文件直连另走 WebRTC，需要 STUN/TURN（见 `RTCPeerConnection` 的 `iceServers`）。
