#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/research-client-record.sh --case NAME --os OS --version VERSION --package PACKAGE

Creates an ignored client validation evidence file under local/research/manual/.
USAGE
}

case_name=""
os_name=""
client_version=""
package_type=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --case)
      case_name="${2:-}"
      shift 2
      ;;
    --os)
      os_name="${2:-}"
      shift 2
      ;;
    --version)
      client_version="${2:-}"
      shift 2
      ;;
    --package)
      package_type="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$case_name" ] || [ -z "$os_name" ] || [ -z "$client_version" ] || [ -z "$package_type" ]; then
  usage >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mkdir -p local/research/manual
safe_case="$(printf "%s" "$case_name" | tr -cs 'A-Za-z0-9._-' '-' | sed 's/^-//; s/-$//')"
stamp="$(date -u +%Y%m%d-%H%M%S)"
target="local/research/manual/client-${stamp}-${safe_case}.md"

cat >"$target" <<EOF
# Client Validation Record

Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Tester:
Endpoint OS/version: ${os_name}
RustDesk client version: ${client_version}
Package type: ${package_type}
Installer source:
Signature/checksum result:
Server target: [redacted]
Commands/scripts used:
Config method:
Config persistence result:
Service/user context result:
Restart result:
Reinstall/upgrade result:
Deploy endpoint result:
Artifacts:
Final status: not-run
Follow-up:

## Step Notes

- Install:
- Configure:
- Readback:
- Restart:
- Reinstall or upgrade:
- Deploy endpoint:

## Redaction Check

- [ ] No production hostnames, IPs, keys, tokens, endpoint IDs, user names, or screenshots are copied into public docs.
- [ ] Any raw artifacts are stored under ignored local/research/.
EOF

printf '%s\n' "$target"
