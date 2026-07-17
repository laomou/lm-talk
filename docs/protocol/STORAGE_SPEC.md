# LM Talk 存储规格 v1

Web 存储使用 IndexedDB 表。敏感字段在应用层使用 AES-GCM 加密，加密密钥由归一化提示词和 UserID 通过 PBKDF2 派生。

静态加密字段包括消息文本/信封、联系人显示名/名片、群名/策略、outbox 载荷、ratchet 会话状态、好友请求原文/发送者名片/备注、群邀请原文/群名、群 Sender Key 状态/分发文本、Mailbox 失败队列载荷/原因，以及同步服务地址/令牌配置。为保证 UI 可用，UserID、group_id、message_id、时间戳、状态、计数等最小路由/索引字段保持明文。

Schema migration 必须兼容旧 localStorage 和单对象 IndexedDB 导入。分表加载时单条联系人、消息、群、outbox 或 ratchet 记录损坏不得阻断整个身份登录；客户端应跳过损坏记录、加载其余数据，并向用户提示可从数据备份或同步服务恢复缺失内容。用户重加密身份备份/修改提示词后，客户端必须用新提示词重新派生本地存储密钥并重写已保存的敏感 IndexedDB 字段。

删除本地身份时，客户端必须清理该 user_id 前缀下的 IndexedDB 分表记录，避免仅删除登录入口但保留本机加密聊天数据。

旧 localStorage 迁移到 IndexedDB 分表时，必须先成功写入新分表再删除原始 localStorage 状态，避免迁移中断造成数据丢失。
