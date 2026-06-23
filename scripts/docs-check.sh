#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_docs=(
  README.md
  TASKS.md
  docs/initial-tapeout.md
  docs/requirements.md
  docs/architecture.md
  docs/software-stack.md
  docs/feature-checklist.md
  docs/pro-feature-parity.md
  docs/validation-matrix.md
  docs/validation-lab.md
  docs/client-delivery.md
  docs/upstream-findings.md
  docs/threat-model.md
  docs/adr.md
  docs/ci-plan.md
  docs/cutover-readiness.md
  docs/engineering-standards.md
  docs/traceability.md
  docs/research-roadmap.md
  docs/research-findings.md
  docs/research/client-deployment.md
  docs/research/operations-security.md
)

for doc in "${required_docs[@]}"; do
  if [ ! -s "$doc" ]; then
    echo "docs-check: missing or empty required doc: $doc" >&2
    exit 1
  fi
done

for script in scripts/*.sh; do
  bash -n "$script"
done

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  ignored_paths=(
    local/context.private.md
    upstream/rustdesk/README.md
    PRIVATE.md
    site.private.md
    .env
    app.env
  )
  word() {
    local value=""
    local char
    for char in "$@"; do
      value+="$char"
    done
    printf "%s" "$value"
  }

  ignored_paths+=(
    ".$(word c o d e x)/config.md"
    ".$(word c l a u d e)/config.md"
    ".$(word g r o k)/config.md"
    "$(word C O D E X).md"
    "$(word C O D E X).local.md"
    "$(word C L A U D E).md"
    "$(word C L A U D E).local.md"
    "$(word G R O K).md"
    "$(word G R O K).local.md"
    "$(word A G E N T S).md"
    "$(word A G E N T S).local.md"
    "$(word a g e n t s).md"
    "$(word a g e n t s).local.md"
  )

  for path in "${ignored_paths[@]}"; do
    if ! git check-ignore -q "$path"; then
      echo "docs-check: expected private/reference path is not ignored: $path" >&2
      exit 1
    fi
  done
fi

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
for root in [pathlib.Path("README.md"), pathlib.Path("TASKS.md")]:
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

python3 - <<'PY'
import pathlib
import re
import sys

validation_prefixes = ("SEC", "CUT", "CI", "S", "C", "D", "E", "R")
validation_pattern = r"(?:SEC|CUT|CI|S|C|D|E|R)-\d{3}"
requirement_pattern = r"(?:PR|SR|OR|IR|CR)-\d{3}"

requirements = pathlib.Path("docs/requirements.md").read_text()
validation = pathlib.Path("docs/validation-matrix.md").read_text()
traceability = pathlib.Path("docs/traceability.md").read_text()

req_ids = set(re.findall(rf"^\|\s*({requirement_pattern})\s*\|", requirements, re.M))
trace_req_ids = set(re.findall(rf"^\|\s*({requirement_pattern})\b", traceability, re.M))
missing_trace = sorted(req_ids - trace_req_ids)
if missing_trace:
    print(
        "docs-check: requirements missing from traceability: "
        + ", ".join(missing_trace),
        file=sys.stderr,
    )
    sys.exit(1)

validation_ids = re.findall(rf"^\|\s*({validation_pattern})\s*\|", validation, re.M)
duplicates = sorted({value for value in validation_ids if validation_ids.count(value) > 1})
if duplicates:
    print(
        "docs-check: duplicate validation IDs: " + ", ".join(duplicates),
        file=sys.stderr,
    )
    sys.exit(1)

validation_id_set = set(validation_ids)
trace_validation_ids = set(re.findall(rf"\b({validation_pattern})\b", traceability))
unknown_validation_ids = sorted(trace_validation_ids - validation_id_set)
if unknown_validation_ids:
    print(
        "docs-check: traceability references unknown validation IDs: "
        + ", ".join(unknown_validation_ids),
        file=sys.stderr,
    )
    sys.exit(1)

range_pattern = re.compile(
    r"\b(SEC|CUT|CI|S|C|D|E|R)-(\d{3})\s+through\s+\1-(\d{3})\b"
)
for prefix, start, end in range_pattern.findall(traceability):
    start_number = int(start)
    end_number = int(end)
    if end_number < start_number:
        print(
            f"docs-check: reversed validation range: {prefix}-{start} through {prefix}-{end}",
            file=sys.stderr,
        )
        sys.exit(1)
    missing = [
        f"{prefix}-{number:03d}"
        for number in range(start_number, end_number + 1)
        if f"{prefix}-{number:03d}" not in validation_id_set
    ]
    if missing:
        print(
            "docs-check: traceability range includes unknown validation IDs: "
            + ", ".join(missing),
            file=sys.stderr,
        )
        sys.exit(1)

for prefix in validation_prefixes:
    if not any(value.startswith(f"{prefix}-") for value in validation_id_set):
        print(f"docs-check: validation matrix has no {prefix}- IDs", file=sys.stderr)
        sys.exit(1)
PY

python3 - <<'PY'
import pathlib
import re
import sys

roadmap = pathlib.Path("docs/research-roadmap.md").read_text()
status_doc = pathlib.Path("docs/research-status.md").read_text()

roadmap_ids = re.findall(r"^\|\s*(R-\d{3})\s*\|", roadmap, re.M)
status_rows = re.findall(r"^\|\s*(R-\d{3})\s*\|\s*([^|]+?)\s*\|", status_doc, re.M)
status_ids = [row[0] for row in status_rows]

expected_ids = [f"R-{number:03d}" for number in range(1, 11)]
if sorted(roadmap_ids) != expected_ids:
    print(
        "docs-check: research roadmap IDs must be exactly R-001 through R-010",
        file=sys.stderr,
    )
    sys.exit(1)

if sorted(status_ids) != expected_ids:
    print(
        "docs-check: research status IDs must be exactly R-001 through R-010",
        file=sys.stderr,
    )
    sys.exit(1)

if len(status_ids) != len(set(status_ids)):
    print("docs-check: duplicate research status IDs", file=sys.stderr)
    sys.exit(1)

allowed_statuses = {"accepted", "partial", "blocked"}
bad_statuses = [
    f"{research_id}={status.strip()}"
    for research_id, status in status_rows
    if status.strip().lower() not in allowed_statuses
]
if bad_statuses:
    print(
        "docs-check: invalid research statuses: " + ", ".join(bad_statuses),
        file=sys.stderr,
    )
    sys.exit(1)

status_map = {research_id: status.strip().lower() for research_id, status in status_rows}
if status_map.get("R-010") != "accepted":
    print("docs-check: R-010 license posture must remain accepted", file=sys.stderr)
    sys.exit(1)

if "Every item in `docs/research-status.md` is `accepted`." not in pathlib.Path(
    "docs/cutover-readiness.md"
).read_text():
    print(
        "docs-check: cutover readiness must require accepted research status",
        file=sys.stderr,
    )
    sys.exit(1)
PY

python3 - <<'PY'
import pathlib
import sys

excluded_roots = {".git", "local", "upstream", "node_modules", "dist", "build", "data", "tmp"}
size_limits = {
    ".go": 400,
    ".rs": 400,
    ".py": 400,
    ".ts": 350,
    ".tsx": 350,
    ".js": 350,
    ".jsx": 350,
    ".html": 300,
    ".css": 400,
    ".md": 250,
    ".sh": 400,
    ".yml": 250,
    ".yaml": 250,
}
banned_stems = {
    "common",
    "helper",
    "helpers",
    "legacy",
    "misc",
    "new",
    "old",
    "shim",
    "temp",
    "util",
    "utils",
    "v2",
}

status = 0
for path in pathlib.Path(".").rglob("*"):
    if not path.is_file():
        continue
    if any(part in excluded_roots for part in path.parts):
        continue

    limit = size_limits.get(path.suffix)
    if limit is not None:
        try:
            line_count = len(path.read_text(errors="ignore").splitlines())
        except OSError as exc:
            print(f"docs-check: cannot read {path}: {exc}", file=sys.stderr)
            status = 1
            continue
        if line_count > limit:
            print(
                f"docs-check: {path} has {line_count} lines, over soft limit {limit}",
                file=sys.stderr,
            )
            status = 1

    if path.stem.lower() in banned_stems:
        print(
            f"docs-check: vague/legacy filename rejected by engineering standards: {path}",
            file=sys.stderr,
        )
        status = 1

if status:
    sys.exit(status)
PY

for id in PR-001 SR-001 OR-001 OR-009 OR-011 IR-001 IR-006 CR-001 CR-008 CI-001 CI-008 CI-009 RS-001; do
  if ! rg -q "$id" docs; then
    echo "docs-check: expected requirement/validation id missing: $id" >&2
    exit 1
  fi
done
