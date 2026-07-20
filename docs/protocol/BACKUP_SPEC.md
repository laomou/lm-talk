# 备份规格 v1

身份备份包是带前缀的文本对象：

```text
lm-identity-backup-v1:<base64url-json>
```

JSON 内容包含：

- `type` / `version`
- `user_id`
- KDF 参数
- 加密后的身份种子
- 创建时间

标准 core/native 路径使用 Argon2id 派生备份密钥，并用 XChaCha20-Poly1305 加密身份种子。提示词错误和备份损坏在内部可以是不同错误，但 UI 应避免泄露过多安全细节。

## 浏览器兼容格式

Web/WASM 还支持本地兼容前缀：

```text
lm-identity-backup-v1:wasm-local:<base64url-json>
```

该格式用于保证浏览器可创建身份，不要求所有浏览器运行时都支持 Argon2id。它仍要求：

- 使用 Web RNG 生成身份种子；
- 使用归一化提示词参与本地密钥派生；
- 用 AEAD 加密种子；
- 恢复时校验 User ID 一致性。

## 文件建议

- 推荐扩展名：`.lmid`
- 推荐 MIME：`application/vnd.lmtalk.identity-backup`

测试向量：`test-vectors/backup_v1.json`。
