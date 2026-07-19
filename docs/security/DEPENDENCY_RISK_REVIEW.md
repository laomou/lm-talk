# 依赖风险复核

本文档跟踪依赖安全例外以及判定漏洞依赖在 LM Talk 中是否可达的过程。它补充 `scripts/check-audit.sh`、CI `dependency-audit`、GitHub `dependency-review` 和 `docs/RELEASE_RISK_REGISTER.md`。

## 当前审计门禁

| 生态 | 命令 / CI | 发布证据 |
| --- | --- | --- |
| Rust | `./scripts/check-audit.sh` 运行 `cargo audit --deny warnings` | CI `dependency-audit` 日志或本地输出 |
| Web npm | `npm audit --audit-level high` 在 `apps/web` 中 | CI `dependency-audit` 日志或本地输出 |
| PR 依赖差异 | GitHub `dependency-review` | PR 检查状态 |

`SKIP_CARGO_AUDIT=1` 仅用于没有 `cargo-audit` 的环境；不得作为发布证据使用。

## 当前忽略的 Rust advisory

`scripts/check-audit.sh` 当前忽略以下 advisory。这些例外在依赖或启用特性更改时必须重新评估。

| Advisory | 当前理由 | 可达性假设 | 何时重新评估 | 发布状态 |
| --- | --- | --- | --- | --- |
| `RUSTSEC-2026-0118` | 由未使用的可选 `libp2p` DNS/mDNS 依赖元数据拉入的传递 `hickory-proto` advisory。 | LM Talk 仅启用 libp2p TCP/noise/yamux/request-response；未启用 DNS/mDNS 功能。 | `libp2p` 升级、启用 DNS/mDNS 功能或 advisory 范围更改时。 | 仅在有文档化的 CI 审计输出时允许例外。 |
| `RUSTSEC-2026-0119` | 同上 `hickory-proto` 依赖系列。 | 同上。 | 同上。 | 仅在有文档化的 CI 审计输出时允许例外。 |
| `RUSTSEC-2024-0436` | 通过传递的 Linux netlink/proc-macro 路径引入的 `paste` 警告，目前与 LM Talk 运行时协议无直接安全相关性。 | LM Talk 的 Web/原生节点控制面、Mailbox/DHT 解析、密码学操作未直接依赖 `paste` 行为。 | netlink 栈或依赖 crate 进入暴露的节点控制/数据路径；依赖升级或 advisory 更改时。 | 仅在有文档化的 CI 审计输出时允许例外。 |

## 新 advisory 的复核工作流

1. 确定直接或传递依赖及启用的特性路径。
2. 判定漏洞代码是否可从以下路径触达：
   - Web/WASM 边界；
   - 原生节点控制面；
   - Mailbox/DHT 解析；
   - 密码学操作；
   - 部署/构建/发布工具链。
3. 如果可达且可利用，则在修复或缓解前视为发布阻塞。
4. 如果不可达，则在本文件中记录特性/路径理由，并为最窄范围的 `cargo audit --ignore` 添加条目。
5. 添加后续项以在依赖升级时重新评估该例外。
6. 将决策链接到 `docs/RELEASE_RISK_REGISTER.md`，当严重性为 medium 或更高时。

## 依赖更新策略

- 优先删除未使用的依赖特性，而不是添加审计例外。
- 保持 `libp2p` 特性最小：macros、noise、request-response、json、tcp、tokio、yamux。
- 保持 SQLCipher 特性显式；除非发布策略更改，否则不要默认为所有产物启用。
- 对于 Web 依赖，避免添加会执行不信任 HTML/markdown 或扩大浏览器权限面的运行时包，除非经过审查。
- Dependabot PR 应包含 CI `dependency-review` 状态和安全相关发布说明影响。

## 发布证据要求

每个发布候选都应归档：

- CI `dependency-audit` 任务日志；
- 如果本地运行，则归档 `./scripts/check-audit.sh` 输出；
- 本文件中主动生效的 `cargo audit --ignore` 例外列表；
- 依赖更改 PR 的 `dependency-review` 状态；
- 任何 Accepted 的依赖风险已复制到 `docs/RELEASE_RISK_REGISTER.md`。

## 否决标准

如果满足以下任一条件，则生产发布为 **NO-GO**：

- 可达的高/严重 advisory 未解决。
- 审计例外缺少可达性理由。
- `npm audit --audit-level high` 未通过且无文档化 Accepted 风险。
- `cargo audit --deny warnings` 对未明确在此处审查的 advisory 失败。
