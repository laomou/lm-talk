# LM Talk 好友请求规格 v1

好友请求文本：

```text
lm-friend-request-v1:<base64url-json>
lm-friend-response-v1:<base64url-json>
```

请求包含发送方联系人名片、目标 UserID、可选备注、创建时间、过期时间和身份签名。响应会对请求 id、发送方/接收方 ID、接受/拒绝结果和创建时间签名。

请求和响应可以手动复制、显示为二维码文本、通过 WebRTC 发送，或作为 Mailbox 的 `Other` 载荷投递。

测试向量见 `test-vectors/friend_request_v1.json`。
