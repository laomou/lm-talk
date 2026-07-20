# LM Talk 三节点 federation 模板

该模板会在本机启动 3 个 `lm_node` 实例，并在默认 compose 模式下通过 Caddy 反向代理暴露端口。它用于在公网部署前验证 Mailbox、DHT 快照同步和副本复制行为。

## 端口

- node A：`http://localhost:8081`
- node B：`http://localhost:8082`
- node C：`http://localhost:8083`

每个节点都会通过 `sync_peers` 与另外两个节点同步。

## 快速开始

```bash
cd deploy/lm-node-federation
mkdir -p secrets
for n in a b c; do
  openssl rand -base64 32 > "secrets/node-$n-token"
done
chmod 600 secrets/*
docker compose up -d --build
./smoke-test.sh
# 或运行完整验证套件：
./run-all.sh
```

如果当前 Docker 安装没有 `docker compose` 插件，可以用脚本内置的 direct `docker run` fallback 做本地验证：

```bash
LM_NODE_FEDERATION_DIRECT_DOCKER=1 ./run-all.sh
```

direct fallback 会把三个节点直接映射到 8081/8082/8083，自动生成缺失的 secret，把本地数据放到 `.docker-data/`，并关闭后台 sync/DHT runner；smoke 脚本会显式导入快照来验证同步链路。

Web 应用的同步设置里需要填写节点 URL，以及 `secrets/node-*-token` 中对应节点的 token。

## 清理本地测试环境

停止并清理容器、网络、本地数据和默认报告：

```bash
./compose.sh clean
```

默认会保留 `secrets/`，方便复用节点 token。若要连 secret 一起删除：

```bash
LM_NODE_FEDERATION_CLEAN_SECRETS=1 ./compose.sh clean
```

## 公网部署注意事项

真实公网节点至少需要：

1. 把 `Caddyfile.*` 中的 `:80` 替换为真实 HTTPS 域名。
2. 把每个 `sync_peers[].url` 改成 peer 的公网 HTTPS URL。
3. 使用唯一 peer ID、唯一 token，并用全盘加密或目录加密保护明文 `state_db` 持久化卷。
4. 把 `cors_allow_origins` 限制为实际部署的 Web / admin 来源。

该模板只用于 federation/bootstrap 测试。长期运行的节点应持续监控 `/health`、`/control/stats` 和 `/control/metrics`。

## Smoke 测试

`./smoke-test.sh` 会检查三个节点的 `/health` 和 `/control/stats`，在节点容器内生成已签名的 smoke identity 和 ContactCard，通过 node A 发布并查询 ContactCard DHT 记录，向 node A 推送已签名 Mailbox 消息，导出 node A 快照并导入 node B，然后验证 node B 能查询 DHT 记录、取回 Mailbox 消息，并检查 sync 状态。

脚本使用 `lm_node identity` / `lm_node contact-card` 生成真实签名的 ContactCard，因此也会覆盖服务端 ContactCard DHT 校验。

## Chaos smoke

`./chaos-smoke.sh` 会模拟 node B 短暂离线：停止 node B，向 node A 推送 5 条已签名 Mailbox 消息，恢复 node B，导入 node A 快照，然后验证 node B 可以取回恢复后的 mailbox 消息。它是短功能性 chaos 检查，不能替代长时间 load/partition 测试。

## Load smoke

`./load-smoke.sh` 会向 node A 推送一小批已签名 Mailbox 消息，导入 node A 快照到 node C，验证 node C mailbox 状态，取回这些消息，并检查 metrics 暴露正常。可通过 `MESSAGE_COUNT=100` 或其他值调整短突发规模。

## 运行完整验证

`./run-all.sh` 默认会启动 stack，除非设置 `LM_NODE_FEDERATION_SKIP_UP=1`。它会把机器可读 JSON 报告写入 `federation-report.json`，也可用 `LM_NODE_FEDERATION_REPORT` 指定路径，然后依次运行：

1. `./smoke-test.sh`
2. `./chaos-smoke.sh`
3. `./load-smoke.sh`

示例：

```bash
./run-all.sh
MESSAGE_COUNT=100 ./run-all.sh
LM_NODE_FEDERATION_SKIP_UP=1 ./run-all.sh
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json ./run-all.sh
```

任一步骤失败时，`run-all.sh` 会立即写出 failed 报告，并打印当前容器状态和最近日志，便于直接定位失败点。

## GitHub Actions

`.github/workflows/federation-smoke.yml` 提供手动 `workflow_dispatch` 任务，会生成新的节点 secret、运行 `./run-all.sh`、上传 `federation-report.json` artifact，并在失败时输出 compose 日志。它适合在常规 quick CI 之外做更重的 federation 验证。
