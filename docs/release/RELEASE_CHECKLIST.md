# 发布检查清单

本文记录 LM Talk 当前功能目标下的发布检查步骤。当前目标不再把第三方审计、长时间 fuzz、公网长期报告、macOS 公证、Windows 签名和风险登记签核作为阻塞项；这些内容保留为可选的生产发行增强。

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
- fuzz harness 编译检查
- Web 类型检查、生产构建和 Playwright 端到端测试

如需更完整的本地检查：

```bash
./scripts/release-check.sh full
```

如需短时 fuzz smoke：

```bash
./scripts/release-check.sh fuzz-smoke
```

## 依赖检查

依赖审计命令：

```bash
./scripts/check-audit.sh
```

当前审计例外仅用于已知不可达或低相关的传递依赖路径。升级 `libp2p`、启用新特性或新增网络解析路径时，应重新检查这些例外。

## Docker / federation 功能测试

三节点 federation 模板位于：

```text
deploy/lm-node-federation/
```

完整 smoke / chaos / load：

```bash
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json \
  deploy/lm-node-federation/run-all.sh
```

该测试会验证：

- 三个节点 `/health` 与 `/control/stats`
- ContactCard DHT 发布与查找
- Mailbox push/take
- snapshot export/import
- node outage 后恢复
- 短 burst Mailbox 负载

## 原生节点产物

推送 `v*` 标签时，`.github/workflows/release-node.yml` 会构建：

- `lm_node-linux-x86_64.tar.gz`
- `lm_node-macos-x86_64.tar.gz`
- `lm_node-macos-arm64.tar.gz`
- `lm_node-windows-x86_64.zip`

每个包应包含：

- `lm_node` 二进制
- 关键部署文档
- `RELEASE_INFO.txt`
- `.sha256`

本地打包示例：

```bash
cargo build --locked --release -p lm_node --target x86_64-unknown-linux-gnu
python3 scripts/release-package.py \
  --target x86_64-unknown-linux-gnu \
  --package-name lm_node-linux-x86_64 \
  --out-dir dist
```

验证已发布 tag：

```bash
./scripts/release-verify.sh v0.1.0
```

## 可选生产发行增强

以下项目不再作为当前功能目标阻塞项，但如果要做公开生产发行，仍建议保留证据：

- 外部安全审计报告
- 长时间 fuzz campaign 与 crash triage
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

3. 运行 Docker federation：

```bash
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json \
  deploy/lm-node-federation/run-all.sh
```

4. 如需发布原生节点产物，打 tag：

```bash
git tag -a v0.1.0 -m "LM Talk node v0.1.0"
git push origin v0.1.0
```

5. 验证 release assets：

```bash
./scripts/release-verify.sh v0.1.0
```
