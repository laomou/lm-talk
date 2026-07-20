# 依赖风险复核

本文记录 LM Talk 的依赖审计方式和当前例外。当前功能目标不把依赖风险签核作为阻塞项，但日常开发仍应运行审计并避免引入可达高危依赖。

## 常用检查

| 生态 | 命令 / 检查 | 说明 |
| --- | --- | --- |
| Rust | `./scripts/check-audit.sh` | 运行 `cargo audit`，并应用仓库记录的窄范围例外。 |
| Web npm | `npm audit --audit-level high` | 在 `apps/web` 中检查高危 npm advisory。 |
| PR 依赖差异 | GitHub dependency review | 检查新增依赖是否带入已知风险。 |

`SKIP_CARGO_AUDIT=1` 只适合没有 `cargo-audit` 的开发环境，不应用作正式验证结果。

## 当前 Rust advisory 例外

`scripts/check-audit.sh` 中忽略的 advisory 必须保持窄范围。若依赖、特性或运行路径发生变化，应重新评估。

| Advisory | 当前理由 | 可达性假设 | 重新评估条件 |
| --- | --- | --- | --- |
| `RUSTSEC-2026-0118` | 来自未启用的可选 `libp2p` DNS/mDNS 依赖元数据。 | LM Talk 当前只启用 libp2p TCP/noise/yamux/request-response。 | 升级 `libp2p`、启用 DNS/mDNS，或 advisory 范围变化。 |
| `RUSTSEC-2026-0119` | 同上，属于 `hickory-proto` 相关传递依赖。 | 同上。 | 同上。 |
| `RUSTSEC-2024-0436` | `paste` 由传递 Linux netlink/proc-macro 路径带入，与当前暴露协议路径无直接关系。 | Web/WASM、节点控制面、Mailbox/DHT 解析和密码学操作不直接依赖其运行时行为。 | netlink 栈进入暴露控制/数据路径，或 advisory 变化。 |

## 新 advisory 复核流程

1. 确认依赖是直接依赖还是传递依赖。
2. 确认触发漏洞的 feature 是否启用。
3. 判断是否能从以下路径触达：
   - Web/WASM 边界；
   - 原生节点控制面；
   - Mailbox/DHT 解析；
   - 密码学操作；
   - 构建/部署工具链。
4. 若可达，优先升级或删除对应依赖/feature。
5. 若不可达，记录原因，并将 ignore 控制到最小范围。
6. 依赖升级后重新检查。

## 依赖更新原则

- 优先减少 feature，而不是扩大忽略列表。
- 保持 `libp2p` feature 最小化。
- Web 依赖避免引入执行不可信 HTML/Markdown 或扩大浏览器权限面的运行时包。
- 新增依赖应说明用途和安全边界。

## 当前目标下的使用方式

依赖审计用于提高质量和安全性，但不再作为当前功能目标的完成阻塞项。若要做公开发行，可在发布证据中附上审计日志和本文件复核结果。
