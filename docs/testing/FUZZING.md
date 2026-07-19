# 模糊测试

LM Talk 提供 `cargo-fuzz` / libFuzzer 脚手架，用于不信任的协议和节点输入。这些 harness 不替代外部安全审计，但应作为生产发布门禁的一部分。

## 前提条件

```bash
cargo install cargo-fuzz
```

## Smoke 检查

在发布候选前，验证所有 fuzz harness 都能启动并执行至少少量输入：

```bash
./scripts/fuzz-smoke.sh
```

这仅是 harness 启动 smoke 测试。它不替代长期 fuzz 活动。

## 目标

```bash
# Core 文本导入解析：contact/friend/backup/prekey/signal/mailbox/message receipt/group/ratchet/file/device revoke
./scripts/fuzz-run.sh core_imports -- -max_total_time=60

# 原生节点 DHT RPC JSON 和快照合并输入
./scripts/fuzz-run.sh node_dht_rpc -- -max_total_time=60

# 原生节点控制请求调度，测试任意 method/path/body 分割
./scripts/fuzz-run.sh node_control_request -- -max_total_time=60
```

对于更长的发布候选，请让每个目标运行数小时，并保存语料库，只提交小且有意义的回归种子。任何崩溃或超时都必须在发布前进行分类。

## 当前范围

- `core_imports` 检查所有公共文本导入解析器，包括签名的送达/已读回执、是否接受/拒绝、好友请求、备份、PreKey、Signal、Mailbox、Message、Group、Ratchet、文件、设备撤销等，确保它们不会 panic，并依然依赖大小/签名/格式验证。
- `node_dht_rpc` 练习 `DhtRpcRequest` 反序列化、`NativeNode::handle_dht_rpc`、`DhtRpcResponse` 序列化和 `NodeStateSnapshot` 合并路径。
- `node_control_request` 练习 `NativeNode::handle_control_request`，使用任意 method/path/body 字符串。

## 仍待完成的发布工作

- 从类似生产的捕获中添加持久语料库，剥离机密信息。
- 添加带时间预算和崩溃产物上传的 CI/夜间 fuzz 任务。
- 随着协议格式稳定，添加更底层的解析器/密码学信封 fuzz 目标。
- 在声称生产就绪前运行独立 AFL/libFuzzer 活动和外部安全审计。
