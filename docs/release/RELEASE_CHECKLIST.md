# 发布检查清单


## 必跑检查

从仓库根目录运行：

```bash
./scripts/release-check.sh quick
```

该命令覆盖：

- `cargo fmt --check`
- `lm_core` 单元测试、端到端测试、属性测试和测试向量
- `lm_node` 库测试和二进制测试
- 节点 HTTP 控制面端到端测试
- Web 类型检查、生产构建和 Playwright 端到端测试

如需更完整的本地检查：

```bash
./scripts/release-check.sh full
```


```bash
```

## 依赖检查

依赖审计命令：

```bash
./scripts/check-audit.sh
```

当前审计例外仅用于已知不可达或低相关的传递依赖路径。升级 `libp2p`、启用新特性或新增网络解析路径时，应重新检查这些例外。

## 原生节点产物

推送 `v*` 标签时，`.github/workflows/release-node.yml` 会构建：

- `lm_node-linux-x86_64`
- `lm_node-macos-x86_64`
- `lm_node-macos-arm64`
- `lm_node-windows-x86_64.exe`

每个平台产物直接是对应的 `lm_node` 可执行文件，不再打包 `node.config.example.json`、`node_admin.zip`、文档或构建元数据。每个二进制都有独立的 `.sha256`，Release 还会提供合并的 `SHA256SUMS.txt`。

本地构建示例：

```bash
cargo build --locked --release -p lm_node --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/lm_node lm_node-linux-x86_64
```

配置模板保留在仓库的 `docs/examples/lm-node.config.example.json`。容器部署请使用发布的 GHCR 镜像。

验证已发布 tag：

```bash
./scripts/release-verify.sh v0.1.0
```

## 可选生产发行增强

以下项目不再作为当前功能目标阻塞项，但如果要做公开生产发行，仍建议保留证据：

- 外部安全审计报告
- 长时间公网 federation chaos/load 报告
- 真实公网部署报告
- macOS notarization
- Windows code signing
- 风险登记 owner / release decision / evidence

## 建议发布步骤

1. 确认工作区干净：

```bash
git status --short
```

2. 运行快速检查：

```bash
./scripts/release-check.sh quick
```

3. 如需发布原生节点产物，打 tag：

```bash
git tag -a v0.1.0 -m "LM Talk node v0.1.0"
git push origin v0.1.0
```

4. 验证 release assets：

```bash
./scripts/release-verify.sh v0.1.0
```
