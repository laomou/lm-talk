# 当前待办清单

本文只记录当前功能目标下仍值得继续做的事项。旧版长 TODO 中大量内容已经完成或转移到设计/协议/部署文档，不再作为当前目标阻塞项。

当前目标：做一个**去中心化、端到端加密的即时通讯应用**，优先保证功能可用和安全默认值。

## 当前完成度

| 项目 | 估计 |
| --- | --- |
| 可用 Demo / MVP | 约 98% |
| 功能目标整体 | 约 98% |

## P0：近期优先

### 1. Docker federation 测试稳定化

- [x] 三节点 federation smoke/chaos/load 脚本迁移到 `deploy/lm-node-federation/`。
- [x] Docker federation 已能跑通 basic / chaos / load。
- [x] 在没有 `docker compose` plugin 的环境中支持 direct docker fallback，并在文档中说明。
- [x] `compose.sh clean` 可清理 direct docker fallback 的临时目录和本地测试产物。
- [ ] README 中补充基础镜像拉取失败/限流时的处理方式。

### 2. strict E2EE 体验打磨

- [x] 新身份默认启用 strict E2EE。
- [x] 自动生成本设备 sealed slot 证书。
- [x] 登录/同步前自动修复旧身份缺失的本设备证书。
- [x] 自动分发修复后的设备证书给好友。
- [x] 单聊、文件、群聊、群事件、群邀请、安全会话、回执等路径执行 strict 策略。
- [x] 修复控制消息例外已记录：ContactCard/device-cert 更新和 device revoke。
- [ ] 在更多 UI 位置区分“核心阻断项”和“非阻断新鲜度提醒”。
- [ ] 群成员列表支持按 strict 风险排序。
- [ ] strict E2EE 修复向导减少跳转，提供更连续的修复流程。

### 3. Web 聊天体验

- [x] 消息投递状态统一展示：queued / mailbox / sent / delivered / read / failed。
- [x] 失败/待发送消息提供单条重试/取消入口。
- [x] 文件附件卡片展示发送中、已入队、已接收、已下载等状态。
- [ ] 消息搜索高亮暂不做，后续如需要再排期。
- [ ] 窄屏下优化聊天、通讯录和设置页布局。

## P1：继续增强

### 4. Native node 运维体验

- [x] node-admin 已增加 federation peer health 总览。
- [ ] `/control/stats` 中聚合更清晰的 Mailbox/DHT 摘要。
- [x] Docker federation 失败时自动打印报告、容器状态和日志尾部。
- [x] 补充开发/运维工作流文档，收敛常用命令入口。
- [ ] 部署文档继续减少过时参数和重复内容。

### 5. 文档中文化和去重

- [x] `docs/testing/` 中文化。
- [x] `docs/release/` 改为中文并标记生产证据为可选增强。
- [x] `docs/security/` 主要文档中文化并标记审计为可选增强。
- [x] `docs/deploy/NODE_CONFIG.md` 简化为中文配置说明。
- [x] `docs/protocol/` 主要规格中文扩写。
- [x] `docs/README.md` 中文索引更新。
- [x] `docs/overview/DESIGN.md` 已作为当前实现状态版维护。
- [x] 已修正旧 docs 根路径引用；继续发现过时路径时随改动处理。

## P2：长期增强

### 6. 协议和互操作

- [ ] 进一步收敛旧 DirectEnvelope 兼容路径。
- [ ] 群聊协议继续向更完整的成员变更/历史同步策略演进。
- [ ] 多设备回执状态合并策略继续细化。
- [ ] 更稳定的跨版本迁移说明。

### 7. 产品化体验

- [ ] 首次使用向导更短。
- [ ] 用户可理解的错误码和恢复建议。
- [ ] 更清晰的“节点不可用 / 令牌错误 / 配额已满 / DHT 未发现”分类提示。
- [ ] 多语言文案后续整理。

## 不作为当前目标阻塞项

以下内容可以作为未来生产发行、合规或商业分发目标，不再阻塞当前功能目标：

- 第三方安全审计报告。
- 长时间 fuzz campaign 产物与 crash triage。
- 长时间公网 federation chaos/load 证据。
- 真实公网部署报告。
- macOS notarization / Windows code signing。
- 风险登记 owner / release decision / evidence 完整签核。

## 常用检查命令

```bash
./scripts/release-check.sh quick
PATH="$PWD/.tools/node/bin:$PATH" npm --prefix apps/web run test:e2e
LM_NODE_FEDERATION_REPORT=/tmp/lm-federation-report.json deploy/lm-node-federation/run-all.sh
```
