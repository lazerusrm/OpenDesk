#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

scan_paths=()
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  while IFS= read -r -d '' path; do
    if [ "$path" = "scripts/privacy-scan.sh" ]; then
      continue
    fi
    scan_paths+=("./$path")
  done < <(git ls-files --cached --others --exclude-standard -z)
else
  while IFS= read -r -d '' path; do
    scan_paths+=("$path")
  done < <(find . \
    -path './.git' -prune -o \
    -path './local' -prune -o \
    -path './upstream' -prune -o \
    -path './scripts/privacy-scan.sh' -prune -o \
    -type f -print0)
fi

if [ "${#scan_paths[@]}" -eq 0 ]; then
  exit 0
fi

patterns=(
  'industrialcamera'
  '(^|[^0-9])10\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}([^0-9]|$)'
  '(^|[^0-9])192\.168\.[0-9]{1,3}\.[0-9]{1,3}([^0-9]|$)'
  '(^|[^0-9])172\.(1[6-9]|2[0-9]|3[0-1])\.[0-9]{1,3}\.[0-9]{1,3}([^0-9]|$)'
  'root@192'
  '-----BEGIN [A-Z ]*PRIVATE KEY-----'
  'PRIVATE KEY-----'
  'password\s*[:=]'
  'secret\s*[:=]'
  'token\s*[:=]'
  'api[_-]?key\s*[:=]'
)

for pattern in "${patterns[@]}"; do
  if rg -n --pcre2 -- "$pattern" "${scan_paths[@]}"; then
    echo "privacy-scan: matched sensitive pattern: $pattern" >&2
    status=1
  fi
done

exit "$status"
