# LM Talk Node 容器镜像

本目录包含公开 `lm_node` HTTP 服务的容器构建定义。

发布镜像：

```text
ghcr.io/laomou/lm-talk-node
```

发布版本 tag 提供 `linux/amd64` 镜像；稳定版本还会更新 `:latest`。

## 使用发布镜像

从仓库中的 `docs/examples/lm-node.config.example.json` 创建 `node.json`，配置
高强度控制 token 后启动：

```bash
docker run --rm \
  --name lm-talk-node \
  -p 8787:8787 \
  -v "$PWD/node.json:/app/config.json:ro" \
  -v "$PWD/lm-node-data:/data" \
  ghcr.io/laomou/lm-talk-node:latest
```

长期部署应固定版本，例如 `ghcr.io/laomou/lm-talk-node:0.1.0`，不要依赖
`:latest`。

服务状态保存在 `/data`；请挂载持久卷，并在宿主机层保护磁盘数据。不要暴露未鉴权的
控制面：必须在 `node.json` 中配置 `control_token_file`（或 `control_token`）。

容器默认执行：

```text
lm_node serve-control --config-file /app/config.json
```
