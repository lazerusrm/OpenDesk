#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "public-content-scan: must run inside a git worktree" >&2
  exit 1
fi

status=0

while IFS= read -r path; do
  case "$path" in
    .gitignore|scripts/public-content-scan.sh)
      continue
      ;;
  esac

  if rg -n -i '\b(ai|agent|agents|agentic|subagent|codex|claude|grok|gpt|chatgpt|openai)\b' "$path"; then
    echo "public-content-scan: blocked tool/automation attribution in $path" >&2
    status=1
  fi
done < <(git ls-files)

exit "$status"
