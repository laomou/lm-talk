# 发布签名与公证

本文件记录 LM Talk 原生 `lm_node` 产物的生产信任分发要求。仅靠校验和有助于完整性，但在 macOS 和 Windows 平台信任方面，校验和本身并不是充分证明。

## 当前状态

- Linux 产物：仅校验和 (`SHA256SUMS.txt` 和每个产物的 `.sha256`)。未来可选硬化：minisign/cosign。
- macOS 产物：**尚未进行 Developer ID 签名或公证**。
- Windows 产物：**尚未进行 Authenticode / Azure Trusted Signing 签名**。

发布工作流会为每个原生节点包生成 `*-signing-evidence.json`。在未实施并验证真实签名/公证前，这些报告会故意标记生产分发就绪为 `false`。

## 所需生产证据

### macOS

在生产信任 macOS 发布前必须具备：

1. 对 `lm_node` 二进制/归档内容进行 Developer ID Application 签名。
2. 对发布产物或打包应用提交 Apple notarization。
3. 在适用场景下执行 Stapling。
4. 将验证日志与发布一起归档：

```bash
codesign --verify --deep --strict --verbose=2 path/to/lm_node
spctl --assess --type execute --verbose path/to/lm_node
xcrun notarytool log <submission-id>
```

预期的发布证据字段：

```json
{
  "macos_codesigned": true,
  "macos_notarized": true,
  "macos_stapled": true
}
```

### Windows

在生产信任 Windows 发布前必须具备：

1. 使用组织控制的代码签名证书或 Azure Trusted Signing 生成 Authenticode 签名。
2. 添加时间戳签名。
3. 将验证日志与发布一起归档：

```powershell
signtool verify /pa /v lm_node.exe
Get-AuthenticodeSignature .\lm_node.exe
```

预期的发布证据字段：

```json
{
  "windows_signed": true,
  "windows_signature_verified": true
}
```

## CI 密钥占位符

不要提交签名凭据。请使用 GitHub Actions secrets 或外部签名服务。

建议为未来实现使用的 secret 名称：

| Secret | 用途 |
| --- | --- |
| `APPLE_TEAM_ID` | Apple Developer Team ID。 |
| `APPLE_NOTARY_ISSUER_ID` | notarytool 的 App Store Connect issuer ID。 |
| `APPLE_NOTARY_KEY_ID` | App Store Connect key ID。 |
| `APPLE_NOTARY_PRIVATE_KEY` | App Store Connect 私钥。 |
| `MACOS_DEVELOPER_ID_CERT_P12` | Base64 编码的 Developer ID 证书。 |
| `MACOS_DEVELOPER_ID_CERT_PASSWORD` | 证书密码。 |
| `WINDOWS_SIGNING_CERT_PFX` | Base64 编码的 Authenticode 证书（如不使用 Azure Trusted Signing）。 |
| `WINDOWS_SIGNING_CERT_PASSWORD` | Windows 代码签名证书密码。 |
| `AZURE_TRUSTED_SIGNING_*` | Azure Trusted Signing 身份/配置（如使用）。 |

## 发布门禁

`docs/RELEASE_RISK_REGISTER.md` 中的 `RISK-005` 在签名/公证证据附加并审查前仍为 `Open`。当 macOS/Windows 签名报告不完整时，生产发布门禁必须保持阻塞。
