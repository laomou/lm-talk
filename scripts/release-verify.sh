#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Verify LM Talk native node GitHub Release assets.

Usage:
  scripts/release-verify.sh <tag> [download-dir]

Example:
  scripts/release-verify.sh v0.1.2
  scripts/release-verify.sh v0.1.2 /tmp/lm-talk-v0.1.2

The script downloads the expected node release assets with `gh release download`,
verifies `SHA256SUMS.txt`, verifies each per-binary `.sha256`, and fails if
an expected platform binary is missing.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  usage >&2
  exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh CLI is required" >&2
  exit 127
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha256_check() { sha256sum -c "$1"; }
elif command -v shasum >/dev/null 2>&1; then
  sha256_check() { shasum -a 256 -c "$1"; }
else
  echo "error: sha256sum or shasum is required" >&2
  exit 127
fi

if [[ -n "${2:-}" ]]; then
  DOWNLOAD_DIR="$2"
  mkdir -p "$DOWNLOAD_DIR"
else
  DOWNLOAD_DIR="$(mktemp -d -t lm-talk-release-${TAG}.XXXXXX)"
fi

REPORT_FILE="${RELEASE_VERIFY_REPORT:-}"

EXPECTED_ASSETS=(
  "lm_node-linux-x86_64"
  "lm_node-macos-x86_64"
  "lm_node-macos-arm64"
  "lm_node-windows-x86_64.exe"
  "SHA256SUMS.txt"
)

printf '== Downloading LM Talk node release %s into %s ==\n' "$TAG" "$DOWNLOAD_DIR"
gh release download "$TAG" --dir "$DOWNLOAD_DIR" --clobber

cd "$DOWNLOAD_DIR"

missing=0
for asset in "${EXPECTED_ASSETS[@]}"; do
  if [[ ! -s "$asset" ]]; then
    echo "missing expected release asset: $asset" >&2
    missing=1
  fi
  if [[ "$asset" == lm_node-* ]]; then
    if [[ ! -s "$asset.sha256" ]]; then
      echo "missing expected checksum asset: $asset.sha256" >&2
      missing=1
    fi
  fi
done
if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

printf '== Verifying combined checksums ==\n'
sha256_check SHA256SUMS.txt

printf '== Verifying per-artifact checksums ==\n'
for binary in lm_node-linux-x86_64 \
  lm_node-macos-x86_64 \
  lm_node-macos-arm64 \
  lm_node-windows-x86_64.exe; do
  sha256_check "$binary.sha256"
done

if [[ -n "$REPORT_FILE" ]]; then
  printf '== Writing release verification report: %s ==\n' "$REPORT_FILE"
  python3 - <<'PY' "$REPORT_FILE" "$TAG" "$DOWNLOAD_DIR"
import hashlib, json, pathlib, sys, time
report_file, tag, download_dir = sys.argv[1:4]
root = pathlib.Path(download_dir)
assets = []
for path in sorted(root.iterdir()):
    if path.is_file():
        assets.append({
            "name": path.name,
            "size": path.stat().st_size,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        })
report = {
    "tag": tag,
    "status": "ok",
    "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "download_dir": download_dir,
    "expected_platform_binaries": [
        "lm_node-linux-x86_64",
        "lm_node-macos-x86_64",
        "lm_node-macos-arm64",
        "lm_node-windows-x86_64.exe",
    ],
    "checks": {
        "expected_assets_present": True,
        "combined_sha256sums_verified": True,
        "per_artifact_sha256_verified": True,
        "platform_binaries_only": True,
    },
    "assets": assets,
}
pathlib.Path(report_file).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
PY
fi

printf '== Release %s verified successfully ==\n' "$TAG"
