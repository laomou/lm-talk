# 开发与运维工作流

本文把仓库常用开发、验证、节点运行和打包命令收在一起。更细的协议字段见 `docs/protocol/`，节点配置字段见 `docs/deploy/NODE_CONFIG.md`。

## 推荐阅读顺序

1. `docs/README.md`：文档总索引。
2. `docs/overview/DESIGN.md`：当前实现总览。
3. 本文：本地开发、验证和节点启动命令。
4. `docs/deploy/NODE_CONFIG.md`：节点配置、`/admin/` 和公网部署注意事项。

## 环境约定

仓库脚本默认在仓库根目录运行。Web / node-admin 命令建议带上本仓库自带 Node：

```bash
export PATH="$PWD/.tools/node/bin:$PATH"
```

若本机没有 `.tools/node`，也可以使用系统 Node，只要版本满足项目依赖。

## 常用本地验证

### Web

```bash
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/web run typecheck
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/web run test:e2e
```

### node-admin

```bash
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/node-admin run typecheck
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/node-admin run test:e2e
```

### 快速发布检查

```bash
./scripts/release-check.sh quick
```

`quick` 覆盖 Rust 格式/测试、WASM/Web 构建、Web E2E、节点测试和 fuzz harness 编译检查。完整发布前可看 `docs/release/RELEASE_CHECKLIST.md`。

## 运行本地节点

### 本机节点 + 内嵌 `/admin/`

```bash
./scripts/dev-run.sh node --local
```

脚本会：

1. 构建 `target/release/lm_node`；
2. 检查 `apps/node-admin/dist` 是否适合挂载到 `/admin/`；
3. 必要时用 `NODE_ADMIN_BASE=/admin/` 自动构建 node-admin；
4. 启动 `lm_node serve-control` 并自动挂载管理页。

打开：

```text
http://127.0.0.1:8787/admin/
```

`/admin/` 是 loopback-only。远程服务器上访问时建议使用 SSH 隧道：

```bash
ssh -L 8787:127.0.0.1:8787 user@server
```

然后在本机浏览器打开 `http://127.0.0.1:8787/admin/`。

### 局域网节点

局域网监听必须配置 token：

```bash
mkdir -p .local
openssl rand -base64 32 > .local/lm-node.token
chmod 600 .local/lm-node.token

./scripts/dev-run.sh node \
  --lan \
  --control-token-file .local/lm-node.token \
  --cors-allow-origin "http://127.0.0.1:4173"
```

`--lan` 会绑定 `0.0.0.0:8787`，但内嵌 `/admin/` 仍然只允许本机 loopback。若要在局域网另一台设备看 UI，请用独立 node-admin 前端。

### 独立 node-admin 前端

```bash
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/node-admin run dev -- --host 0.0.0.0 --port 4173
```

打开：

```text
http://<你的局域网 IP>:4173/
```

在页面里填写节点地址和 token：

```text
http://<节点局域网 IP>:8787
```

## Docker federation 本地验证

如果本机有 `docker compose`：

```bash
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json \
  deploy/lm-node-federation/run-all.sh
```

如果没有 compose plugin，可用 direct docker fallback：

```bash
LM_NODE_FEDERATION_DIRECT_DOCKER=1 \
MESSAGE_COUNT=10 \
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json \
  deploy/lm-node-federation/run-all.sh
```

清理容器、网络和本地测试数据：

```bash
deploy/lm-node-federation/compose.sh clean
```

默认保留 `secrets/`。如需连 secret 一起删除：

```bash
LM_NODE_FEDERATION_CLEAN_SECRETS=1 deploy/lm-node-federation/compose.sh clean
```

## 产物构建

### 节点 release 二进制

```bash
./scripts/dev-build.sh node
```

如果后续要构建 `deploy/lm-node-public/Dockerfile` 或发布容器包，仓库会从 Docker Hub 拉取 `rust:1-bookworm` 作为 builder 基础镜像。若本机遇到拉取失败或 429 限流，通常可先重试、登录 Docker、换成已缓存的本地镜像，或在联网更稳定时重新构建。

### Web 生产包

```bash
./scripts/dev-build.sh web
```

### 全量本地产物

```bash
./scripts/dev-build.sh all
```

该命令会构建 release `lm_node`、Web 生产包和用于 `/admin/` 的 node-admin 静态包。

### node-admin `/admin/` 静态包

推荐使用脚本：

```bash
./scripts/dev-build.sh node-admin
```

等价手工命令：

```bash
PATH="$PWD/.tools/node/bin:$PATH" \
NODE_ADMIN_BASE=/admin/ \
  npm --prefix apps/node-admin run build
```

`node-admin` 挂到 `lm_node --web-admin` 时必须使用 `/admin/` base。`scripts/dev-run.sh node --local` 会自动处理这件事。

## 生成物与缓存

常见生成物：

| 路径 | 说明 | 是否应提交 |
| --- | --- | --- |
| `target/` | Rust 构建输出 | 否 |
| `apps/web/dist/` | Web 生产包 | 否 |
| `apps/node-admin/dist/` | node-admin 静态包 | 否 |
| `.local/` | 本地 token / 临时配置 | 否 |
| `deploy/lm-node-federation/.docker-data/` | federation 测试数据 | 否 |
| `deploy/lm-node-federation/.docker-run/` | direct docker 运行配置 | 否 |
| `deploy/lm-node-federation/secrets/` | federation 测试 token | 否 |
| `node_admin.zip` | release 包内嵌 `/admin/` 管理页 | 不提交；`release-package.py` 自动生成或打包 |

提交前建议：

```bash
git status --short
git diff --check
```

## 推荐提交节奏

- 一个功能或一类文档扫尾对应一个 commit。
- 每次提交前运行与改动相关的最小验证。
- 不把本地 token、数据库、Docker 数据目录和 release artifact 直接提交。
- GitHub CI / Pages 状态不作为本地继续开发的阻塞，除非发布任务明确要求等待。
