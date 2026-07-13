# LM Talk Network Spec v1

Networking is optional and best-effort.

- WebRTC DataChannel is preferred for online direct delivery.
- Mailbox is used for offline delivery through configured `lm_node` control plane.
- Outbox retries use exponential backoff: 30s, 2m, 10m, 1h, 6h; default expiry is 7 days.
- Snapshot sync can pull `/sync/snapshot` from a peer node and import it locally.

Current node networking is a control-plane scaffold with closest-peer and snapshot support, not a production DHT.

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

