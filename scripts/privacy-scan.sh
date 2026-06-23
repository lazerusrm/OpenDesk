#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

scan_paths=()
while IFS= read -r -d '' path; do
  scan_paths+=("$path")
done < <(find . \
  -path './.git' -prune -o \
  -path './local' -prune -o \
  -path './upstream' -prune -o \
  -path './scripts/privacy-scan.sh' -prune -o \
  -path './CODEX.md' -prune -o \
  -path './docs/ci-plan.md' -prune -o \
  -type f -print0)

if [ "${#scan_paths[@]}" -eq 0 ]; then
  exit 0
fi

patterns=(
  'industrialcamera'
  '192\.168\.0\.100'
  'root@192'
  'BEGIN .*PRIVATE KEY'
  'PRIVATE KEY-----'
  'password\s*='
  'secret\s*='
  'token\s*='
  'api[_-]?key\s*='
)

for pattern in "${patterns[@]}"; do
  if rg -n --pcre2 "$pattern" "${scan_paths[@]}"; then
    echo "privacy-scan: matched sensitive pattern: $pattern" >&2
    status=1
  fi
done

exit "$status"
