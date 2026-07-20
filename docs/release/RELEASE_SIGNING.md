# 发布签名与校验

本文说明原生 `lm_node` 产物的校验方式和可选平台签名。当前功能目标不要求 macOS notarization 或 Windows code signing。

## 当前校验方式

发布工作流会为原生节点产物生成：

- 每个归档的 `.sha256`
- 合并的 `SHA256SUMS.txt`
- `RELEASE_INFO.txt`
- `*-signing-evidence.json`（用于记录签名/公证状态）

用户可用以下方式验证下载完整性：

```bash
sha256sum -c SHA256SUMS.txt
```

或使用仓库脚本：

```bash
./scripts/release-verify.sh v0.1.0
```

## 平台签名状态

| 平台 | 当前要求 | 说明 |
| --- | --- | --- |
| Linux | SHA256 校验 | 当前功能目标已足够；未来可选 minisign/cosign。 |
| macOS | 可选 | Developer ID 签名和 notarization 不再作为当前目标阻塞项。 |
| Windows | 可选 | Authenticode / Azure Trusted Signing 不再作为当前目标阻塞项。 |

## 可选生产发行增强

如果未来要面向公网用户提供生产信任分发，可补充：

### macOS

```bash
codesign --verify --deep --strict --verbose=2 path/to/lm_node
spctl --assess --type execute --verbose path/to/lm_node
xcrun notarytool log <submission-id>
```

### Windows

```powershell
signtool verify /pa /v lm_node.exe
Get-AuthenticodeSignature .\lm_node.exe
```

## 证据文件

`*-signing-evidence.json` 可记录：

```json
{
  "macos_codesigned": false,
  "macos_notarized": false,
  "windows_signed": false,
  "production_distribution_ready": false
}
```

这些字段用于透明展示状态；当前不作为功能目标阻塞项。
