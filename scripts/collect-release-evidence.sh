#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${RELEASE_EVIDENCE_DIR:-$ROOT/release-evidence}"
VERSION="${RELEASE_VERSION:-unknown}"
COMMIT="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
mkdir -p "$OUT_DIR"

copy_if_exists() {
  local src="$1" name="$2"
  if [[ -f "$src" ]]; then
    cp "$src" "$OUT_DIR/$name"
    printf 'true'
  else
    printf 'false'
  fi
}

release_check_present=$(copy_if_exists "$ROOT/release-check.log" "release-check.log")
fuzz_smoke_present=$(copy_if_exists "$ROOT/fuzz-smoke-report.json" "fuzz-smoke-report.json")
fuzz_campaign_present=$(copy_if_exists "$ROOT/fuzz-campaign-artifacts/fuzz-campaign-report.json" "fuzz-campaign-report.json")
federation_present=$(copy_if_exists "$ROOT/deploy/lm-node-federation/federation-report.json" "federation-report.json")
sqlcipher_present=$(copy_if_exists "$ROOT/sqlcipher-smoke-report.json" "sqlcipher-smoke-report.json")
release_asset_verify_present=$(copy_if_exists "$ROOT/release-asset-verify-report.json" "release-asset-verify-report.json")
risk_register_gate_present=$(copy_if_exists "$ROOT/risk-register-gate.log" "risk-register-gate.log")

python3 - <<'PY' "$OUT_DIR/release-evidence-index.json" "$VERSION" "$COMMIT" "$STARTED_AT" \
  "$release_check_present" "$fuzz_smoke_present" "$fuzz_campaign_present" "$federation_present" "$sqlcipher_present" "$release_asset_verify_present" "$risk_register_gate_present"
import json, sys
(out, version, commit, started_at, release_check, fuzz_smoke, fuzz_campaign, federation, sqlcipher, release_asset_verify, risk_register_gate) = sys.argv[1:]
checks = {
    "release_check_log_present": release_check == "true",
    "fuzz_smoke_report_present": fuzz_smoke == "true",
    "fuzz_campaign_report_present": fuzz_campaign == "true",
    "federation_report_present": federation == "true",
    "sqlcipher_smoke_report_present": sqlcipher == "true",
    "release_asset_verify_report_present": release_asset_verify == "true",
    "risk_register_gate_log_present": risk_register_gate == "true",
}
missing = [name for name, ok in checks.items() if not ok]
report = {
    "version": version,
    "commit": commit,
    "generated_at": started_at,
    "status": "complete" if not missing else "incomplete",
    "checks": checks,
    "missing": missing,
    "notes": "This index only records local artifacts found at collection time. Fill docs/RELEASE_EVIDENCE.md with links to CI artifacts, release assets, audits, and manual approvals before claiming production readiness.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2)
    f.write("\n")
PY

echo "release evidence collected: $OUT_DIR"
python3 -m json.tool "$OUT_DIR/release-evidence-index.json" >/dev/null
