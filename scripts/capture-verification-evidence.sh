#!/usr/bin/env bash
set -uo pipefail

SCRATCH="${1:?scratch directory}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$SCRATCH"

failed=0

run_logged() {
  local log_file=$1
  shift
  "$@" 2>&1 | tee "$log_file"
  local exit_code=${PIPESTATUS[0]}
  echo "exit=$exit_code" | tee -a "$log_file"
  if [ "$exit_code" -ne 0 ]; then
    failed=1
  fi
  return 0
}

append_scan() {
  local name=$1
  shift
  local scan_exit=0
  {
    echo "=== ${name} $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
    "$@" || scan_exit=$?
    echo "${name} exit=${scan_exit}"
    return "$scan_exit"
  } 2>&1 | tee -a "$SCRATCH/scans.log"
  local exit_code=${PIPESTATUS[0]}
  if [ "$exit_code" -ne 0 ]; then
    failed=1
  fi
}

: >"$SCRATCH/scans.log"
append_scan docs-check bash "$REPO/scripts/docs-check.sh"
append_scan privacy-scan bash "$REPO/scripts/privacy-scan.sh"
append_scan public-content-scan bash "$REPO/scripts/public-content-scan.sh"

run_logged "$SCRATCH/cargo-test-1.log" bash -c "cd '$REPO' && cargo test"
run_logged "$SCRATCH/cargo-test-2.log" bash -c "cd '$REPO' && cargo test --quiet"

chmod +x "$REPO/scripts/launch-seed-and-capture.sh"
bash "$REPO/scripts/launch-seed-and-capture.sh" "$SCRATCH" 1
bash "$REPO/scripts/launch-seed-and-capture.sh" "$SCRATCH" 2

{
  echo "branch=$(git -C "$REPO" branch --show-current)"
  echo "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  git -C "$REPO" log --oneline -10
} >"$SCRATCH/git-log.log"

if [ "$failed" -ne 0 ]; then
  echo "capture-verification-evidence: one or more steps failed" >&2
  exit 1
fi

echo "capture-verification-evidence: all steps passed; artifacts in $SCRATCH"