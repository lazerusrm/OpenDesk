#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "public-content-scan: must run inside a git worktree" >&2
  exit 1
fi

status=0
terms=(
  'a''i'
  'a''g''e''n''t'
  'a''g''e''n''t''s'
  'a''g''e''n''t''i''c'
  's''u''b''a''g''e''n''t'
  'a''s''s''i''s''t''a''n''t'
  'c''o''d''e''x'
  'c''l''a''u''d''e'
  'g''r''o''k'
  'g''p''t'
  'c''h''a''t''g''p''t'
  'o''p''e''n''a''i'
  'l''l''m'
  'c''o''p''i''l''o''t'
)
pattern="\\b($(IFS='|'; echo "${terms[*]}"))\\b"

while IFS= read -r path; do
  if rg -n -i "$pattern" "$path"; then
    echo "public-content-scan: blocked private-workflow marker in $path" >&2
    status=1
  fi
done < <(git ls-files)

exit "$status"
