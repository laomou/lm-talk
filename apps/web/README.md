# LM Talk Web

LM Talk 的网页客户端（Vue + PWA）。这里是应用界面；项目整体介绍见根目录 [README](../../README.md)。

## 运行

```bash
cd apps/web
npm install
npm run dev      # 本地开发
npm run build    # 生产构建
```

> 构建会顺带编译 WASM 模块，需预先安装 Rust 与 wasm-pack。

## 功能

- 注册 / 导入 / 导出身份
- 名片、二维码加好友
- 私聊与群聊（端到端加密）
- 连接同步服务后可离线收发、跨设备同步
- 可安装为桌面 / 手机应用（PWA）

## 使用与边界

- 你对自己收发、存储、导出、分享的内容负责。
- 端到端加密保护消息内容，但不隐藏全部元数据（如收发时间、联系关系等）。
- 身份为本地自持：丢失身份备份与提示词将无法找回账号。
- 消息为尽力投递，不保证一定送达或被阅读。

## 许可证

[MIT](../../LICENSE) © 2026 LM Talk contributors
