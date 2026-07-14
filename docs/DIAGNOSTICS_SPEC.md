# 诊断报告规范

版本：v0.1  
日期：2026-07-14  
状态：实现同步草案

本文档定义 Web 诊断报告可以导出的字段、必须禁止的字段，以及用户分享报告前的预览和确认要求。诊断报告用于排查登录、同步、消息收发和本地数据问题，不是备份包，也不是审计日志。

---

## 设计目标

- 帮助用户和开发者定位环境、同步服务、本地队列和存储状态问题。
- 默认只导出状态摘要和计数，不导出聊天内容或密钥材料。
- 复制或分享前必须让用户看到完整报告文本。
- 报告格式稳定，便于人工阅读和自动化解析。

非目标：

- 不作为身份、联系人、消息或节点状态的恢复来源。
- 不保证覆盖所有运行时错误。
- 不上传到默认服务器；分享动作由用户显式触发。

---

## 当前报告格式

Web 当前生成 JSON 文本，顶层字段如下：

```json
{
  "diagnostics_version": 1,
  "report_scope": "full",
  "time": "2026-07-14T00:00:00.000Z",
  "account": {},
  "browser": {},
  "sync": {},
  "local_counts": {},
  "recent_logs": []
}
```

字段语义：

| 字段 | 类型 | 说明 |
|---|---|---|
| `diagnostics_version` | number | 诊断报告 schema 版本；当前为 `1`。 |
| `report_scope` | string | 报告范围，`full` 表示完整状态摘要，`summary` 表示仅摘要。 |
| `time` | string | 生成报告的 ISO 8601 时间。 |
| `account` | object | 当前账号摘要。 |
| `browser` | object | 浏览器能力和本地缓存能力摘要。 |
| `sync` | object | 节点同步配置和连接状态摘要。 |
| `local_counts` | object | 本地对象数量统计。 |
| `recent_logs` | string[] | 最近 UI 日志摘要，必须经过敏感信息约束。 |

---

## 允许字段

### 账号摘要

允许：

- `user_id`
- `display_name`

约束：

- `user_id` 和 `display_name` 可能识别用户身份，报告 UI 必须在复制前展示完整文本。
- 仅摘要模式不得包含 `account`。
- 不得包含身份私钥、身份种子、设备私钥、PreKey 私钥、Sender Key 或备份包内容。

### 浏览器能力

允许：

- 是否为安全上下文。
- 是否支持 IndexedDB。
- 是否支持 WebCrypto。
- 是否支持 Service Worker。
- Service Worker 注册数量。
- Cache Storage cache key 列表。

约束：

- Cache key 不应包含消息内容、联系人名称、密钥材料或完整远端 URL token。
- 后续如果 cache key 含用户可控输入，需要先做脱敏或改为数量统计。

### 同步状态

允许：

- 是否启用同步服务。
- 已配置同步服务 URL 列表。
- 当前连接状态。

约束：

- 同步服务 URL 不得包含 token、basic auth、一次性凭据或其他认证秘密。
- 如果未来支持带凭据 URL，诊断报告必须只展示 origin 或经过脱敏后的 URL。

### 本地计数

允许：

- 联系人数。
- 群组数。
- 好友请求数。
- 群邀请数。
- outbox 总数。
- 未完成 outbox 数。
- 本地消息数。

约束：

- 只允许计数，不允许导出联系人列表、群成员列表、消息列表或附件元数据列表。

### 最近日志

允许：

- UI 层状态变化摘要。
- 网络请求成功/失败摘要。
- 同步、发送、接收、解密失败的错误类别。

约束：

- 单条日志不得包含提示词、备份包、私钥、seed、原始消息明文、附件内容、完整 direct envelope、完整 mailbox payload、完整 Contact Card、完整 Friend Request 或 auth token。
- 错误对象需要转换为稳定错误类别或短文本，不直接序列化异常对象。
- 默认最多导出最近 12 条。
- 仅摘要模式不得包含 `recent_logs`。

---

## 禁止字段

诊断报告任何位置都不得包含：

- 用户输入的提示词或提示词派生材料。
- `identity_seed`、身份私钥、设备私钥、X25519 私钥、Ed25519 私钥。
- signed prekey 私钥、one-time prekey 私钥。
- Sender Key、ratchet root key、chain key、message key、skipped keys。
- 身份备份包全文或本地加密身份 blob。
- 消息明文、附件明文、文件内容、缩略图内容。
- 完整密文消息 payload、完整 mailbox payload 或完整 outbox payload。
- 控制面 token、URL 中的 token、basic auth、cookie、localStorage/IndexedDB 原始 dump。
- 浏览器指纹级别的高熵信息，除非有明确排障必要并单独提示。

---

## 分享流程

Web 诊断分享必须满足：

1. 用户点击“生成诊断报告”后才生成报告。
2. 报告以可读 JSON 展示在页面中。
3. 用户点击“复制报告”前可以完整预览。
4. 复制按钮只复制当前预览文本，不后台追加隐藏字段。
5. UI 文案必须说明报告不包含提示词、身份私钥或消息明文。
6. 仅摘要模式生成的预览文本必须只包含浏览器能力、同步状态和本地计数，不得在复制时追加隐藏字段。

后续增强：

- 当前 Web 已提供脱敏开关，可隐藏 `user_id`、`display_name` 和同步服务 URL。
- 当前 Web 已提供仅摘要模式，只生成并复制浏览器能力、同步状态和本地计数。

---

## 实现映射

当前 Web 实现：

- 页面：`apps/web/src/components/DiagnosticsPage.vue`
- 入口：设置页“诊断工具”
- 当前导出：状态摘要 JSON、最近日志、手动复制

实现要求：

- 新增诊断字段时必须先更新本文档。
- 新增日志来源时必须确认日志不会写入禁止字段。
- 如果未来支持上传诊断报告，必须新增单独的用户确认和目的地展示。
