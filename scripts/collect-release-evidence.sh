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

copy_signing_reports() {
  local count=0
  shopt -s nullglob
  for src in "$ROOT"/*-signing-evidence.json "$ROOT"/signing-evidence.json; do
    if [[ -f "$src" ]]; then
      cp "$src" "$OUT_DIR/$(basename "$src")"
      count=$((count + 1))
    fi
  done
  shopt -u nullglob
  printf '%s' "$count"
}

release_check_present=$(copy_if_exists "$ROOT/release-check.log" "release-check.log")
fuzz_smoke_present=$(copy_if_exists "$ROOT/fuzz-smoke-report.json" "fuzz-smoke-report.json")
fuzz_campaign_present=$(copy_if_exists "$ROOT/fuzz-campaign-artifacts/fuzz-campaign-report.json" "fuzz-campaign-report.json")
federation_present=$(copy_if_exists "$ROOT/deploy/lm-node-federation/federation-report.json" "federation-report.json")
sqlcipher_present=$(copy_if_exists "$ROOT/sqlcipher-smoke-report.json" "sqlcipher-smoke-report.json")
release_asset_verify_present=$(copy_if_exists "$ROOT/release-asset-verify-report.json" "release-asset-verify-report.json")
risk_register_gate_present=$(copy_if_exists "$ROOT/risk-register-gate.log" "risk-register-gate.log")
risk_register_gate_report_present=$(copy_if_exists "$ROOT/risk-register-gate-report.json" "risk-register-gate-report.json")
signing_report_count=$(copy_signing_reports)

python3 - <<'PY' "$OUT_DIR/release-evidence-index.json" "$VERSION" "$COMMIT" "$STARTED_AT" \
  "$release_check_present" "$fuzz_smoke_present" "$fuzz_campaign_present" "$federation_present" "$sqlcipher_present" "$release_asset_verify_present" "$risk_register_gate_present" "$risk_register_gate_report_present" "$signing_report_count"
import json, pathlib, sys
(out, version, commit, started_at, release_check, fuzz_smoke, fuzz_campaign, federation, sqlcipher, release_asset_verify, risk_register_gate, risk_register_gate_report, signing_report_count) = sys.argv[1:]
checks = {
    "release_check_log_present": release_check == "true",
    "fuzz_smoke_report_present": fuzz_smoke == "true",
    "fuzz_campaign_report_present": fuzz_campaign == "true",
    "federation_report_present": federation == "true",
    "sqlcipher_smoke_report_present": sqlcipher == "true",
    "release_asset_verify_report_present": release_asset_verify == "true",
    "risk_register_gate_log_present": risk_register_gate == "true",
    "risk_register_gate_report_present": risk_register_gate_report == "true",
    "signing_evidence_reports_present": int(signing_report_count) > 0,
}
missing = [name for name, ok in checks.items() if not ok]
risk_gate_status = "missing"
risk_gate_issue_count = None
risk_gate_counts = None
risk_gate_report = pathlib.Path(out).with_name("risk-register-gate-report.json")
risk_gate_log = pathlib.Path(out).with_name("risk-register-gate.log")
if risk_gate_report.exists():
    parsed = json.loads(risk_gate_report.read_text(encoding="utf-8"))
    risk_gate_status = parsed.get("status") or "unknown"
    risk_gate_issue_count = len(parsed.get("issues") or [])
    risk_gate_counts = parsed.get("counts")
elif risk_gate_log.exists():
    text = risk_gate_log.read_text(encoding="utf-8", errors="replace")
    for line in text.splitlines():
        if line.startswith("status="):
            risk_gate_status = line.split("=", 1)[1].strip() or "unknown"
            break
    if risk_gate_status == "missing":
        risk_gate_status = "unknown"
    risk_gate_issue_count = sum(1 for line in text.splitlines() if line.startswith("- RISK-"))
signing_reports = []
for path in pathlib.Path(out).parent.glob("*-signing-evidence.json"):
    signing_reports.append(json.loads(path.read_text(encoding="utf-8")))
for path in pathlib.Path(out).parent.glob("signing-evidence.json"):
    signing_reports.append(json.loads(path.read_text(encoding="utf-8")))
signing_summary = {
    "reports": len(signing_reports),
    "macos_notarized": any((r.get("checks") or {}).get("macos_notarized") for r in signing_reports),
    "windows_signed": any((r.get("checks") or {}).get("windows_signed") for r in signing_reports),
    "production_distribution_ready": bool(signing_reports) and all(r.get("production_distribution_ready") is True for r in signing_reports),
}
production_gate = {
    "risk_register_gate_status": risk_gate_status,
    "risk_register_gate_issue_count": risk_gate_issue_count,
    "risk_register_gate_counts": risk_gate_counts,
    "risk_register_gate_report_present": risk_register_gate_report == "true",
    "risk_register_production_ready": risk_gate_status == "ok",
    "signing": signing_summary,
}
report = {
    "version": version,
    "commit": commit,
    "generated_at": started_at,
    "status": "complete" if not missing else "incomplete",
    "checks": checks,
    "missing": missing,
    "production_gate": production_gate,
    "notes": "This index only records local artifacts found at collection time. Fill docs/RELEASE_EVIDENCE.md with links to CI artifacts, release assets, audits, and manual approvals before claiming production readiness.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2)
    f.write("\n")
PY

echo "release evidence collected: $OUT_DIR"
python3 -m json.tool "$OUT_DIR/release-evidence-index.json" >/dev/null
