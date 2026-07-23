# 局域网 HTTPS 部署

局域网部署由两个 Docker 容器组成：

- `lm-talk-web`：Caddy 提供 HTTPS、Web 静态页面，并代理 `/node/*`；
- `lm-talk-node`：带 Bearer token 保护的同步服务，仅在 Docker 网络中监听。

## 前提

- 已安装 Docker；从源码更新 Node 时还需要 Rust；
- 局域网设备可以访问启动机器的 TCP `443`；
- `lm-talk-web` Caddy 与 `lm-talk-node` 位于同一个 Docker 网络；
- Windows 等客户端已信任 Caddy `tls internal` 的根证书。

## HTTPS 路由

浏览器只访问 Caddy。这里的 `<HTTPS-主机>` 是**本次部署自己的地址**，不是固定
`10.235.112.40`：可以是当前服务器的局域网 IP，也可以是如 `lm-talk.lan` 的局域网名称。

```bash
https://<HTTPS-主机>/
```

同步接口同样通过 Caddy：

```text
https://<HTTPS-主机>/node|<control-token>
```

`lm-talk-web` 的 Caddyfile 需要包含：

```caddy
https://<HTTPS-主机> {
  tls internal

  handle_path /node/* {
    reverse_proxy lm-talk-node:8787
  }

  handle_path /admin/* {
    root * /admin
    try_files {path} /index.html
    file_server
  }

  handle {
    root * /srv
    try_files {path} /index.html
    file_server
  }
}
```

Node 不发布 `8787` 到宿主机；它只通过 `lm-talk-web` 的 Caddy 接收请求。

同一个 Caddy 容器还提供独立的 Node 管理前端：

```text
https://<HTTPS-主机>/admin/
```

它是 Caddy 提供的静态前端，不是 `lm_node` 的 `/admin` 路径；页面默认连接同源
`/node` API，仍须输入控制面 token。

## 从源码启动 Web / Caddy

首次部署或更新 Web 时执行：

```bash
./scripts/dev-run.sh web \
  --public-url https://<HTTPS-主机> \
  --caddy-data-dir /home/user/lm-talk-web/caddy-data
```

脚本会构建 Web 镜像、启动 `lm-talk-web`、发布宿主机 `80/443`，并自动生成包含
`/node/* → lm-talk-node:8787` 与 `/admin/` 管理前端的 Caddyfile。重复使用同一个
`--caddy-data-dir`，Caddy 会复用其中已有的内部 CA 和 HTTPS 证书：

```text
/home/user/lm-talk-web/caddy-data
```

你当前已导入客户端的根证书可继续使用：

```text
/home/mourui/lm-talk-web/lm-talk-local-root.crt
```

脚本不会重新生成或覆盖它。已有自定义 Caddyfile 时可挂载它：

```bash
./scripts/dev-run.sh web \
  --caddyfile /home/user/lm-talk-web/Caddyfile \
  --caddy-data-dir /home/user/lm-talk-web/caddy-data
```

## 从源码更新 Node

```bash
# 使用已有 Node 配置与数据目录，重建镜像并替换容器
./scripts/dev-run.sh node \
  --config-file /home/user/lm-talk-node/config.json \
  --data-dir /home/user/lm-talk-node/data \
  --public-url https://<HTTPS-主机>

# 仅重新创建现有镜像对应的容器
./scripts/dev-run.sh node \
  --config-file /home/user/lm-talk-node/config.json \
  --data-dir /home/user/lm-talk-node/data \
  --public-url https://<HTTPS-主机> \
  --no-build
```

配置中的 `cors_allow_origins` 必须包含 Web 的 HTTPS 来源，例如：

```json
{
  "cors_allow_origins": ["https://<HTTPS-主机>"]
}
```

`control_token` 等同于同步服务访问密码，不要发送到不可信渠道。更多配置项见 [NODE_CONFIG.md](NODE_CONFIG.md)。
