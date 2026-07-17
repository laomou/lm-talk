#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REGISTER="${1:-$ROOT/docs/RELEASE_RISK_REGISTER.md}"
MODE="${RISK_REGISTER_GATE_MODE:-strict}"

usage() {
  cat <<'USAGE'
Usage:
  scripts/risk-register-gate.sh [risk-register.md]

Environment:
  RISK_REGISTER_GATE_MODE=strict|report

Strict mode is the production release gate: every Medium/High/Critical risk
must have an owner and release decision; Open/Rejected High/Medium/Critical
risks fail the gate; Accepted risks must link mitigation/evidence and include a
release decision. Report mode prints the same findings but exits 0.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

python3 - <<'PY' "$REGISTER" "$MODE"
import re
import sys
from pathlib import Path

register_path = Path(sys.argv[1])
mode = sys.argv[2]
if mode not in {"strict", "report"}:
    raise SystemExit(f"invalid RISK_REGISTER_GATE_MODE={mode!r}; expected strict or report")
if not register_path.exists():
    raise SystemExit(f"risk register not found: {register_path}")

lines = register_path.read_text(encoding="utf-8").splitlines()
rows = []
for line in lines:
    stripped = line.strip()
    if not stripped.startswith("| RISK-"):
        continue
    cells = [cell.strip() for cell in stripped.strip("|").split("|")]
    if len(cells) != 7:
        raise SystemExit(f"malformed risk row with {len(cells)} cells: {line}")
    rows.append(dict(zip(["id", "risk", "severity", "status", "mitigation", "owner", "decision"], cells)))

if not rows:
    raise SystemExit("no RISK-* rows found in risk register")

allowed_severities = {"Low", "Medium", "High", "Critical"}
allowed_statuses = {"Open", "Mitigated", "Accepted", "Rejected", "Closed"}
blocking_severities = {"Medium", "High", "Critical"}
blocking_statuses = {"Open", "Rejected"}
issues = []

for row in rows:
    rid = row["id"]
    severity = row["severity"]
    status = row["status"]
    owner = row["owner"]
    decision = row["decision"]
    mitigation = row["mitigation"]

    if severity not in allowed_severities:
        issues.append(f"{rid}: invalid severity {severity!r}")
    if status not in allowed_statuses:
        issues.append(f"{rid}: invalid status {status!r}")

    if severity in blocking_severities:
        if not owner:
            issues.append(f"{rid}: {severity} risk is missing Owner")
        if not decision:
            issues.append(f"{rid}: {severity} risk is missing Release decision")

    if severity in blocking_severities and status in blocking_statuses:
        issues.append(f"{rid}: {severity} risk is {status}; production release is no-go")

    if severity == "Critical" and status == "Accepted":
        issues.append(f"{rid}: Critical risk cannot be accepted")

    if status in {"Accepted", "Mitigated"}:
        if not mitigation or mitigation in {"-", "TODO", "TBD"}:
            issues.append(f"{rid}: {status} risk must link mitigation/evidence")
        if status == "Accepted" and not decision:
            issues.append(f"{rid}: Accepted risk must include release-note/acceptance decision")

print(f"risk_register={register_path}")
print(f"mode={mode}")
print(f"risks={len(rows)}")
if issues:
    print("status=blocked")
    print("issues:")
    for issue in issues:
        print(f"- {issue}")
    if mode == "strict":
        sys.exit(1)
else:
    print("status=ok")
PY
