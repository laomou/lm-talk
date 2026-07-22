# LM Talk Web 容器镜像

本目录包含静态浏览器应用的容器构建定义。最终镜像使用 Caddy 在 `80` 端口提供
编译后的 Web 文件。

发布镜像：

```text
ghcr.io/laomou/lm-talk-web
```

发布版本 tag 会同时提供 `linux/amd64` 与 `linux/arm64` 镜像；稳定版本还会更新
`:latest`。

## 使用发布镜像

```bash
docker run --rm \
  --name lm-talk-web \
  -p 8080:80 \
  ghcr.io/laomou/lm-talk-web:latest
```

访问 `http://127.0.0.1:8080`。公网部署时，应将此容器置于 HTTPS 反向代理之后。
浏览器应用依赖安全上下文中的 WebCrypto，因此正式部署必须使用 HTTPS。

同步服务地址和 token 在 Web 应用中配置；Web 镜像不包含节点凭据。

长期部署应固定版本，例如 `ghcr.io/laomou/lm-talk-web:0.1.0`，不要依赖
`:latest`。
