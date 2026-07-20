#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REGISTER="${1:-$ROOT/docs/release/RELEASE_RISK_REGISTER.md}"
MODE="${RISK_REGISTER_GATE_MODE:-strict}"
REPORT="${RISK_REGISTER_GATE_REPORT:-}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/release-risk-gate.sh [risk-register.md]

Environment:
  RISK_REGISTER_GATE_MODE=strict|report
  RISK_REGISTER_GATE_REPORT=path/to/risk-register-gate-report.json

Strict mode is the production release gate: every Medium/High/Critical risk
must have an owner, release decision, evidence requirement, and evidence link;
Open/Rejected High/Medium/Critical risks fail the gate. Report mode prints the
same findings but exits 0. When RISK_REGISTER_GATE_REPORT is set, a JSON report
is written for release evidence automation.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

python3 - <<'PY' "$REGISTER" "$MODE" "$REPORT"
import json
import sys
import time
from pathlib import Path

register_path = Path(sys.argv[1])
mode = sys.argv[2]
report_path = Path(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] else None
if mode not in {"strict", "report"}:
    raise SystemExit(f"invalid RISK_REGISTER_GATE_MODE={mode!r}; expected strict or report")
if not register_path.exists():
    raise SystemExit(f"risk register not found: {register_path}")

lines = register_path.read_text(encoding="utf-8").splitlines()
header = None
rows = []
for line in lines:
    stripped = line.strip()
    if stripped.startswith("| ID |"):
        raw_header = [cell.strip() for cell in stripped.strip("|").split("|")]
        aliases = {
            "风险 / 限制": "Risk",
            "严重性": "Severity",
            "状态": "Status",
            "当前缓解": "Mitigation / evidence",
            "负责人": "Owner",
            "发布决策": "Release decision",
            "所需证据": "Evidence required",
            "证据链接": "Evidence link",
        }
        header = [aliases.get(cell, cell) for cell in raw_header]
        continue
    if not stripped.startswith("| RISK-"):
        continue
    cells = [cell.strip() for cell in stripped.strip("|").split("|")]
    if header and len(cells) == len(header):
        rows.append(dict(zip(header, cells)))
    elif len(cells) == 7:
        legacy = ["ID", "Risk", "Severity", "Status", "Mitigation / evidence", "Owner", "Release decision"]
        rows.append(dict(zip(legacy, cells)))
    else:
        raise SystemExit(f"malformed risk row with {len(cells)} cells: {line}")

if not rows:
    raise SystemExit("no RISK-* rows found in risk register")

allowed_severities = {"Low", "Medium", "High", "Critical"}
allowed_statuses = {"Open", "Mitigated", "Accepted", "Rejected", "Closed"}
blocking_severities = {"Medium", "High", "Critical"}
blocking_statuses = {"Open", "Rejected"}
issues = []
counts = {
    "risks": len(rows),
    "open_high": 0,
    "open_medium": 0,
    "open_critical": 0,
    "missing_owner": 0,
    "missing_decision": 0,
    "missing_evidence_required": 0,
    "missing_evidence_link": 0,
    "accepted_without_release_note": 0,
    "mitigated_without_artifact": 0,
}

def value(row, key):
    return (row.get(key) or "").strip()

def is_blank(v):
    return not v or v in {"-", "TODO", "TBD"} or v.startswith("TODO(")

for row in rows:
    rid = value(row, "ID")
    severity = value(row, "Severity")
    status = value(row, "Status")
    owner = value(row, "Owner")
    decision = value(row, "Release decision")
    mitigation = value(row, "Mitigation / evidence")
    evidence_required = value(row, "Evidence required")
    evidence_link = value(row, "Evidence link")

    if severity not in allowed_severities:
        issues.append(f"{rid}: invalid severity {severity!r}")
    if status not in allowed_statuses:
        issues.append(f"{rid}: invalid status {status!r}")

    if severity == "High" and status == "Open": counts["open_high"] += 1
    if severity == "Medium" and status == "Open": counts["open_medium"] += 1
    if severity == "Critical" and status == "Open": counts["open_critical"] += 1

    if severity in blocking_severities and mode == "strict":
        if is_blank(owner):
            counts["missing_owner"] += 1
            issues.append(f"{rid}: {severity} risk is missing Owner")
        if is_blank(decision):
            counts["missing_decision"] += 1
            issues.append(f"{rid}: {severity} risk is missing Release decision")
        if is_blank(evidence_required):
            counts["missing_evidence_required"] += 1
            issues.append(f"{rid}: {severity} risk is missing Evidence required")
        if is_blank(evidence_link):
            counts["missing_evidence_link"] += 1
            issues.append(f"{rid}: {severity} risk is missing Evidence link")

    if severity in blocking_severities and status in blocking_statuses and mode == "strict":
        issues.append(f"{rid}: {severity} risk is {status}; production release is no-go")

    if severity == "Critical" and status == "Accepted":
        issues.append(f"{rid}: Critical risk cannot be accepted")

    if status in {"Accepted", "Mitigated"} and mode == "strict":
        if is_blank(mitigation):
            issues.append(f"{rid}: {status} risk must link mitigation/evidence")
        if is_blank(evidence_link):
            issues.append(f"{rid}: {status} risk must link evidence artifact")
        if status == "Accepted" and "release" not in decision.lower():
            counts["accepted_without_release_note"] += 1
            issues.append(f"{rid}: Accepted risk must include release-note/acceptance wording")
        if status == "Mitigated" and not any(token in evidence_link.lower() for token in ["commit", "report", "artifact", "log", "json", "http", "docs/"]):
            counts["mitigated_without_artifact"] += 1
            issues.append(f"{rid}: Mitigated risk must link a commit/test/report/artifact")

status = "blocked" if issues else "ok"
report = {
    "risk_register": str(register_path),
    "mode": mode,
    "status": status,
    "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "counts": counts,
    "issues": issues,
    "risks": [
        {
            "id": value(row, "ID"),
            "severity": value(row, "Severity"),
            "status": value(row, "Status"),
            "owner": value(row, "Owner"),
            "release_decision": value(row, "Release decision"),
            "evidence_required": value(row, "Evidence required"),
            "evidence_link": value(row, "Evidence link"),
        }
        for row in rows
    ],
}

print(f"risk_register={register_path}")
print(f"mode={mode}")
print(f"risks={len(rows)}")
print(f"status={status}")
if issues:
    print("issues:")
    for issue in issues:
        print(f"- {issue}")
if report_path:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"report={report_path}")
if issues and mode == "strict":
    sys.exit(1)
PY
