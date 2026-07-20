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
verifies `SHA256SUMS.txt`, verifies each per-artifact `.sha256`, and fails if
an expected platform archive is missing.
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
  "lm_node-linux-x86_64.tar.gz"
  "lm_node-macos-x86_64.tar.gz"
  "lm_node-macos-arm64.tar.gz"
  "lm_node-windows-x86_64.zip"
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
for archive in lm_node-linux-x86_64.tar.gz \
  lm_node-macos-x86_64.tar.gz \
  lm_node-macos-arm64.tar.gz \
  lm_node-windows-x86_64.zip; do
  sha256_check "$archive.sha256"
done


printf '== Verifying archive contents ==\n'
python3 - <<'PYCHECK'
import pathlib, tarfile, zipfile
archives = [
    pathlib.Path('lm_node-linux-x86_64.tar.gz'),
    pathlib.Path('lm_node-macos-x86_64.tar.gz'),
    pathlib.Path('lm_node-macos-arm64.tar.gz'),
    pathlib.Path('lm_node-windows-x86_64.zip'),
]
for archive in archives:
    if archive.suffix == '.zip':
        with zipfile.ZipFile(archive) as zf:
            names = zf.namelist()
            info = next((name for name in names if name.endswith('/RELEASE_INFO.txt')), None)
            if not any(name.endswith('/node_admin.zip') for name in names):
                raise SystemExit(f'{archive}: missing node_admin.zip')
            if info is None:
                raise SystemExit(f'{archive}: missing RELEASE_INFO.txt')
            release_info = zf.read(info).decode('utf-8', 'replace')
    else:
        with tarfile.open(archive, 'r:gz') as tf:
            names = tf.getnames()
            info = next((name for name in names if name.endswith('/RELEASE_INFO.txt')), None)
            if not any(name.endswith('/node_admin.zip') for name in names):
                raise SystemExit(f'{archive}: missing node_admin.zip')
            if info is None:
                raise SystemExit(f'{archive}: missing RELEASE_INFO.txt')
            member = tf.extractfile(info)
            release_info = member.read().decode('utf-8', 'replace') if member else ''
    if 'web_admin_bundled=true' not in release_info:
        raise SystemExit(f'{archive}: RELEASE_INFO.txt does not record web_admin_bundled=true')
print('archive contents ok')
PYCHECK

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
    "expected_platform_archives": [
        "lm_node-linux-x86_64.tar.gz",
        "lm_node-macos-x86_64.tar.gz",
        "lm_node-macos-arm64.tar.gz",
        "lm_node-windows-x86_64.zip",
    ],
    "checks": {
        "expected_assets_present": True,
        "combined_sha256sums_verified": True,
        "per_artifact_sha256_verified": True,
        "node_admin_zip_present": True,
    },
    "assets": assets,
}
pathlib.Path(report_file).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
PY
fi

printf '== Release %s verified successfully ==\n' "$TAG"
