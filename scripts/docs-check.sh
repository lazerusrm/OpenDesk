#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_docs=(
  README.md
  CODEX.md
  docs/requirements.md
  docs/architecture.md
  docs/pro-feature-parity.md
  docs/validation-matrix.md
  docs/client-delivery.md
  docs/threat-model.md
  docs/ci-plan.md
  docs/cutover-readiness.md
  docs/engineering-standards.md
  docs/traceability.md
)

for doc in "${required_docs[@]}"; do
  if [ ! -s "$doc" ]; then
    echo "docs-check: missing or empty required doc: $doc" >&2
    exit 1
  fi
done

while IFS= read -r target; do
  target="${target%%#*}"
  if [ -n "$target" ] && [ ! -e "$target" ]; then
    echo "docs-check: broken local markdown link target: $target" >&2
    exit 1
  fi
done < <(
  python3 - "$@" <<'PY'
import pathlib
import re
for root in [pathlib.Path("README.md"), pathlib.Path("CODEX.md"), pathlib.Path("TASKS.md")]:
    paths = [root] if root.exists() else []
    for path in paths:
        for target in re.findall(r"\[[^\]]+\]\(([^)]+)\)", path.read_text()):
            if target.startswith(("http://", "https://", "#")):
                continue
            if ".md" in target:
                print(target)
for path in pathlib.Path("docs").glob("*.md"):
    for target in re.findall(r"\[[^\]]+\]\(([^)]+)\)", path.read_text()):
        if target.startswith(("http://", "https://", "#")):
            continue
        if ".md" in target:
            print(str(path.parent / target))
PY
)

for id in PR-001 SR-001 OR-001 OR-009 IR-001 CR-001 CI-001 CI-008; do
  if ! rg -q "$id" docs; then
    echo "docs-check: expected requirement/validation id missing: $id" >&2
    exit 1
  fi
done
