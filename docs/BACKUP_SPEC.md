# LM Talk 备份规格 v1

身份备份包是带文本前缀的 JSON 对象，JSON 使用无填充 base64url 编码：

```text
lm-identity-backup-v1:<base64url-json>
```

JSON 对象包含 type/version/user_id、Argon2id 参数，以及用 XChaCha20-Poly1305 加密的身份种子。备份 AEAD 的 AAD 由 core 加密模块固定。提示词错误和备份损坏在内部是不同错误，但 UI 应避免泄露过多安全细节。

浏览器 WASM 当前还接受一个本地兼容前缀：

```text
lm-identity-backup-v1:wasm-local:<base64url-json>
```

该路径用于保证 Web 身份创建可用，不要求每个浏览器运行时都支持 Argon2id 备份加密。它仍使用 Web RNG 生成身份种子，使用归一化后的提示词参与本地密钥派生，用 AEAD 加密种子，并在恢复时校验 UserID 一致性。Native/core 备份仍使用上面的标准 Argon2id 格式。

推荐文件扩展名：`.lmid`。
推荐 MIME：`application/vnd.lmtalk.identity-backup`。

测试向量见 `test-vectors/backup_v1.json`。
