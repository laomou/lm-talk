#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/signing-evidence.sh --package-name NAME --target TRIPLE --out FILE

Creates a machine-readable signing/notarization evidence report for a release
artifact. This script currently records production signing readiness and fails
closed: unless a future signing step sets LM_SIGNING_STATUS=signed and provides
verification evidence, macOS notarization and Windows code signing are reported
as incomplete.
USAGE
}

PACKAGE_NAME=""
TARGET=""
OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --package-name) PACKAGE_NAME="${2:-}"; shift 2 ;;
    --target) TARGET="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done
if [[ -z "$PACKAGE_NAME" || -z "$TARGET" || -z "$OUT" ]]; then
  usage >&2
  exit 2
fi

python3 - <<'PY' "$PACKAGE_NAME" "$TARGET" "$OUT"
import json, os, pathlib, sys, time
package_name, target, out = sys.argv[1:4]
is_macos = 'apple-darwin' in target
is_windows = 'windows' in target or 'msvc' in target
is_linux = 'linux' in target
signing_status = os.environ.get('LM_SIGNING_STATUS', 'unsigned')
verification_log = os.environ.get('LM_SIGNING_VERIFICATION_LOG', '')
report = {
    'schema': 'lm-release-signing-evidence-v1',
    'package_name': package_name,
    'target': target,
    'generated_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
    'status': 'complete' if signing_status == 'signed' else 'incomplete',
    'production_distribution_ready': False,
    'platform': {
        'linux': is_linux,
        'macos': is_macos,
        'windows': is_windows,
    },
    'checks': {
        'macos_codesigned': False,
        'macos_notarized': False,
        'macos_stapled': False,
        'windows_signed': False,
        'windows_signature_verified': False,
        'linux_checksum_only': is_linux,
    },
    'verification_log': verification_log or None,
    'notes': [],
}
if is_macos:
    report['notes'].append('macOS artifact is not production-ready until Developer ID signing, notarization, stapling, and verification evidence are attached.')
elif is_windows:
    report['notes'].append('Windows artifact is not production-ready until Authenticode/Azure Trusted Signing evidence and verification logs are attached.')
elif is_linux:
    report['notes'].append('Linux artifact uses checksum verification; consider adding minisign/cosign if a signed Linux distribution channel is required.')
else:
    report['notes'].append('Unknown target platform; signing status must be reviewed manually.')
pathlib.Path(out).parent.mkdir(parents=True, exist_ok=True)
pathlib.Path(out).write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
PY
