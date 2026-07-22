#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RELEASE_VERSION="${RELEASE_VERSION:-preprod-local}"
EVIDENCE_DIR="${RELEASE_EVIDENCE_DIR:-$ROOT/release-evidence}"
RUN_FULL="${RUN_FULL:-1}"
RUN_RELEASE_ASSET_VERIFY="${RUN_RELEASE_ASSET_VERIFY:-0}"
RUN_RISK_REGISTER_GATE="${RUN_RISK_REGISTER_GATE:-1}"
RELEASE_TAG_VERIFY="${RELEASE_TAG_VERIFY:-$RELEASE_VERSION}"

mkdir -p "$EVIDENCE_DIR"

echo "== preprod evidence: $RELEASE_VERSION =="
echo "evidence_dir=$EVIDENCE_DIR"

if [[ "$RUN_FULL" == "1" ]]; then
  echo "== release-check full =="
  ./scripts/release-check.sh full 2>&1 | tee "$ROOT/release-check.log"
else
  echo "== release-check full skipped =="
fi

if [[ "$RUN_RELEASE_ASSET_VERIFY" == "1" ]]; then
  echo "== release asset verification: $RELEASE_TAG_VERIFY =="
  RELEASE_VERIFY_REPORT="$ROOT/release-asset-verify-report.json" \
    ./scripts/release-verify.sh "$RELEASE_TAG_VERIFY" "$EVIDENCE_DIR/release-assets-$RELEASE_TAG_VERIFY"
else
  echo "== release asset verification skipped =="
fi

if [[ "$RUN_RISK_REGISTER_GATE" == "1" ]]; then
  echo "== risk register gate report =="
  RISK_REGISTER_GATE_MODE=report \
    RISK_REGISTER_GATE_REPORT="$ROOT/risk-register-gate-report.json" \
    ./scripts/release-risk-gate.sh 2>&1 | tee "$ROOT/risk-register-gate.log"
else
  echo "== risk register gate skipped =="
fi

echo "== collect release evidence =="
RELEASE_VERSION="$RELEASE_VERSION" RELEASE_EVIDENCE_DIR="$EVIDENCE_DIR" ./scripts/release-evidence.sh

echo "== preprod evidence complete =="
echo "index=$EVIDENCE_DIR/release-evidence-index.json"
