# 模糊测试

LM Talk 提供 `cargo-fuzz` / libFuzzer 脚手架，用于验证不可信协议输入和节点控制面输入不会导致 panic、越界或未受控资源消耗。当前目标以功能可用为主；本文件作为开发测试说明，不再作为当前目标阻塞项。

## 安装前置工具

```bash
cargo install cargo-fuzz
```

## 快速 smoke

发布候选或改动解析器后，先确认所有 fuzz harness 能启动：

```bash
./scripts/fuzz-smoke.sh
```

该命令只证明 harness 可运行，不等价于长时间 fuzz。

## Harness 目标

### `core_imports`

覆盖核心导入解析器：

- 联系人名片
- 好友请求/响应
- 身份备份
- PreKey bundle / signed one-time prekey
- Signal / secure session 文本
- Mailbox message
- Message receipt
- 群邀请 / 群事件 / Sender Key
- Ratchet state / envelope
- 文件包
- 设备证书 / 设备撤销

运行示例：

```bash
./scripts/fuzz-run.sh core_imports -- -max_total_time=60
```

### `node_dht_rpc`

覆盖原生节点 DHT RPC JSON、`NativeNode::handle_dht_rpc`、响应序列化和 snapshot merge 输入。

```bash
./scripts/fuzz-run.sh node_dht_rpc -- -max_total_time=60
```

### `node_control_request`

覆盖原生节点控制请求调度，包括任意 method/path/body 组合。

```bash
./scripts/fuzz-run.sh node_control_request -- -max_total_time=60
```

## 结果处理

- 任何 crash、timeout 或 OOM 都应保存输入样本。
- 先最小化样本，再转成单元测试或回归向量。
- 不提交包含真实身份、token、消息明文或私钥的语料。
- 若样本来自诊断报告，必须先脱敏。

## 当前建议

日常开发至少运行 `./scripts/fuzz-smoke.sh`。长时间 fuzz 可作为未来质量增强项，但不再作为当前功能目标的完成条件。
