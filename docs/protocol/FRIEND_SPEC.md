# 好友请求规格 v1

好友请求和响应都是带前缀的签名文本：

```text
lm-friend-request-v1:<base64url-json>
lm-friend-response-v1:<base64url-json>
```

## 好友请求

请求包含：

- 请求 ID；
- 发送方 ContactCard；
- 目标 User ID；
- 可选备注；
- 创建时间和过期时间；
- 发送方身份签名。

客户端必须检查目标是否为当前身份，并检查签名与过期时间。

## 好友响应

响应绑定：

- 原请求 ID；
- 请求双方 User ID；
- 接受或拒绝结果；
- 创建时间；
- 响应方签名。

## 投递方式

请求和响应可以：

- 手动复制；
- 显示为二维码文本；
- 通过 WebRTC 发送；
- 作为 Mailbox `Other` 载荷投递。

测试向量：`test-vectors/friend_request_v1.json`。
