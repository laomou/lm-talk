# lm_node 性能模拟脚本

`node-perf.sh` 是独立的性能模拟工具，不属于 `cargo test`，也不进入普通 CI。

默认模式会：

1. 编译当前分支的 release `lm_node`；
2. 启动一个临时的 loopback Node 和临时 SQLite state DB；
3. 用压测驱动生成真实签名的 Identity、PreKey、DHT Record 与 MailboxMessage；
4. 请求真实 HTTP 控制面；
5. 将输出写入 `artifacts/perf/<时间>/summary.txt`。

```bash
./scripts/perf/node-perf.sh
```

## 场景

```bash
# 基础控制面：health、PreKey、DHT 写入、snapshot 导出
./scripts/perf/node-perf.sh --scenario api

# 单聊投递：Mailbox push、take、ack、并发发送、Long Poll 唤醒
./scripts/perf/node-perf.sh --scenario chat --messages 2000 --concurrency 16

# api + chat
./scripts/perf/node-perf.sh --scenario mixed
```

## 已有 Node

可以测一个已有的明文 HTTP Node。令牌优先通过环境变量提供，脚本不会输出令牌。

```bash
LM_NODE_PERF_TOKEN='your-token' \
./scripts/perf/node-perf.sh \
  --target http://127.0.0.1:8787 \
  --scenario mixed
```

当前驱动使用原始 TCP HTTP 客户端，故不接受 HTTPS URL。要验证 Caddy/TLS/局域网链路，应使用独立的浏览器或 HTTP 压测工具；本脚本的目标是分离并测量 `lm_node` 控制面本身。

## 指标含义

- `avg`：平均耗时；
- `p50/p95/p99`：分位延迟；
- `max`：最大耗时；
- `msg/s`：并发 Mailbox push 阶段的总吞吐。

结果包含 Node 的请求解析、签名校验、内存状态修改和 SQLite state DB 保存。结果不包含 Web/WASM 加密、Caddy HTTPS 和真实 LAN 网络。
