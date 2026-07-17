# LM Talk 身份规格 v1

身份由 32 字节的 `identity_seed` 派生。

- Ed25519 身份签名密钥：HKDF(identity_seed, `lm-talk.identity.ed25519.v1`)。
- X25519 静态密钥：HKDF(identity_seed, `lm-talk.identity.x25519.v1`)。
- UserID：`lm1_` 加 Ed25519 公钥的稳定 base32/blake3 摘要。
- 本地存储密钥：HKDF(identity_seed, storage context)，只用于本地加密状态。

提示词在任何备份密钥派生前都必须归一化，包括标准 native/core Argon2id 备份格式和浏览器 WASM `wasm-local` 兼容备份格式：

1. Unicode NFKC.
2. 去除首尾空白。
3. 将全角空格转换为 ASCII 空格。
4. 将连续空白折叠为一个 ASCII 空格。
5. 不静默删除正常用户字符。

测试向量见 `test-vectors/identity_v1.json`。
