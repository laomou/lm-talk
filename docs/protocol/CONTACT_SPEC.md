# LM Talk 联系人规格 v1

联系人名片是带签名的身份声明：

```text
lm-contact-card-v1:<base64url-json>
```

签名字段包括 UserID、显示名、身份公钥、X25519 公钥、设备证书、创建时间和可选过期时间。只有当 `user_id` 和 `identity_public_key` 与现有联系人一致时，客户端才可以更新显示名和设备证书。禁止静默替换身份密钥。

信任等级：Imported、LinkImported、QrScanned、FingerprintVerified。
指纹展示使用由 BLAKE3 派生的短十六进制码。

测试向量见 `test-vectors/contact_card_v1.json`。
