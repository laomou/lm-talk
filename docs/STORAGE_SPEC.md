# LM Talk 存储规格 v1

Web 存储使用 IndexedDB 表。敏感字段在应用层使用 AES-GCM 加密，加密密钥由归一化提示词和 UserID 通过 PBKDF2 派生。

静态加密字段包括消息文本、联系人显示名/名片、群名、outbox 载荷和 ratchet 会话状态。为保证 UI 可用，最小路由/索引字段保持明文。

Schema migration 必须兼容旧 localStorage 和单对象 IndexedDB 导入。
