# Release Signing / 发布签名与公证

This document tracks the production-trust distribution requirements for LM Talk native `lm_node` artifacts. Checksums alone are useful for integrity, but they are not a substitute for platform trust on macOS and Windows.

## Current status

- Linux artifacts: checksum-only (`SHA256SUMS.txt` and per-artifact `.sha256`). Optional future hardening: minisign/cosign.
- macOS artifacts: **not yet Developer ID signed or notarized**.
- Windows artifacts: **not yet Authenticode / Azure Trusted Signing signed**.

The release workflow emits `*-signing-evidence.json` for every native node package. Until real signing/notarization is implemented and verified, these reports intentionally mark production distribution readiness as `false`.

## Required production evidence

### macOS

Required before a production-trust macOS release:

1. Developer ID Application signing for the `lm_node` binary/archive contents.
2. Apple notarization submission for the release artifact or packaged app.
3. Stapling where applicable.
4. Verification logs archived with the release:

```bash
codesign --verify --deep --strict --verbose=2 path/to/lm_node
spctl --assess --type execute --verbose path/to/lm_node
xcrun notarytool log <submission-id>
```

Expected release evidence fields:

```json
{
  "macos_codesigned": true,
  "macos_notarized": true,
  "macos_stapled": true
}
```

### Windows

Required before a production-trust Windows release:

1. Authenticode signature using an organization-controlled code-signing certificate or Azure Trusted Signing.
2. Timestamped signature.
3. Verification logs archived with the release:

```powershell
signtool verify /pa /v lm_node.exe
Get-AuthenticodeSignature .\lm_node.exe
```

Expected release evidence fields:

```json
{
  "windows_signed": true,
  "windows_signature_verified": true
}
```

## CI secret placeholders

Do not commit signing credentials. Use GitHub Actions secrets or an external signing service.

Suggested secret names for a future implementation:

| Secret | Purpose |
| --- | --- |
| `APPLE_TEAM_ID` | Apple Developer Team ID. |
| `APPLE_NOTARY_ISSUER_ID` | App Store Connect issuer ID for notarytool. |
| `APPLE_NOTARY_KEY_ID` | App Store Connect key ID. |
| `APPLE_NOTARY_PRIVATE_KEY` | App Store Connect private key. |
| `MACOS_DEVELOPER_ID_CERT_P12` | Base64 encoded Developer ID certificate. |
| `MACOS_DEVELOPER_ID_CERT_PASSWORD` | Certificate password. |
| `WINDOWS_SIGNING_CERT_PFX` | Base64 encoded Authenticode certificate, if not using Azure Trusted Signing. |
| `WINDOWS_SIGNING_CERT_PASSWORD` | Windows certificate password. |
| `AZURE_TRUSTED_SIGNING_*` | Azure Trusted Signing identity/configuration, if used. |

## Release gate

`RISK-005` in `docs/RELEASE_RISK_REGISTER.md` remains `Open` until signed/notarized evidence is attached and reviewed. The production release gate must remain blocked while macOS/Windows signing reports are incomplete.
