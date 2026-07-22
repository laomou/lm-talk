# 局域网一键启动

本脚本会在一台局域网机器上启动两项服务：

- LM Talk Web 静态页面；
- 带 Bearer token 保护的 LM Talk 同步服务。

## 前提

- 已安装 Rust、Node.js/npm、`wasm-pack`、Python 3；
- 局域网设备可以访问启动机器的 TCP `4173` 和 `8787` 端口；
- 防火墙只对可信局域网放行这两个端口。

## 启动

在仓库根目录执行：

```bash
./scripts/lan-start.sh
```

脚本会：

1. 构建 release `lm_node` 和 Web 生产包；
2. 首次启动时，在 `.lan/control.token` 生成随机 token，并设置为仅当前用户可读；
3. 启动 `0.0.0.0:8787` 的同步服务；
4. 启动 `0.0.0.0:4173` 的 Web 静态服务；
5. 输出局域网 Web 地址和可直接粘贴的“同步服务地址”。

例如输出：

```text
Web page:        http://192.168.1.23:4173
Sync address:    http://192.168.1.23:8787|<token>
```

所有设备先打开 `Web page`，再在 **我 → 同步与安全 → 编辑地址** 中粘贴 `Sync address`，保存后开启同步。

按 `Ctrl+C` 同时停止 Web 和同步服务。节点状态存放在 `.lan/lm-node.sqlite3`；不要随意删除该文件。`control.token` 等同于同步服务访问密码，不要发到不可信渠道。

## 常用选项

```bash
# 使用其他端口
./scripts/lan-start.sh --web-port 8080 --node-port 8788

# 把状态和 token 放在指定目录
./scripts/lan-start.sh --state-dir /srv/lm-talk

# 已构建过时，直接启动
./scripts/lan-start.sh --no-build
```

当前脚本使用 Python 的静态文件服务，适合可信局域网试用。长期运行、跨网段或公网部署请使用反向代理和 HTTPS，并参考 [NODE_CONFIG.md](NODE_CONFIG.md)。
