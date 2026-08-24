#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/validation-evidence-record.sh --case NAME --platform PLATFORM \
  --client-version VERSION [--package PACKAGE] [--enrollment STATUS] \
  [--server-config STATUS] [--access STATUS] [--wan STATUS] \
  [--nat-relay STATUS] [--update STATUS] [--rollback STATUS]

Creates a redacted validation checklist under ignored local/research/manual/.
PLATFORM is one of: windows, linux, macos, android, ios.
STATUS is one of: not-run, pass, fail, accepted-exception.
This recorder accepts metadata and statuses only; never pass tokens, keys,
passwords, hostnames, addresses, or command output as arguments.
USAGE
}

case_name=""
platform=""
client_version=""
package_type="not-specified"
enrollment="not-run"
server_config="not-run"
access="not-run"
wan="not-run"
nat_relay="not-run"
update="not-run"
rollback="not-run"

valid_status() {
  case "$1" in
    not-run|pass|fail|accepted-exception) return 0 ;;
    *) return 1 ;;
  esac
}

reject_sensitive_or_topology() {
  local value="${1,,}"
  [[ "$value" != *[!A-Za-z0-9._-]* ]] || return 1
  [[ ! "$value" =~ (token|secret|password|passwd|credential|bearer|private|public[-_.]?key|host(name)?|domain|address|(^|[-_.])ip($|[-_.])|hbbs|hbbr|relay|server|api) ]]
}

valid_case_name() {
  [[ "$1" =~ ^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$ ]] && reject_sensitive_or_topology "$1"
}

valid_client_version() {
  [[ "$1" =~ ^v?[0-9]+(\.[0-9]+){1,3}(([-+])[A-Za-z0-9][A-Za-z0-9.-]{0,15})?$ ]] || return 1
  [[ ! "$1" =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]] || return 1
  reject_sensitive_or_topology "$1"
}

valid_package_type() {
  case "$1" in
    installer|portable|deb|rpm|appimage|flatpak|dmg|apk|android|ios|not-specified) return 0 ;;
    *) return 1 ;;
  esac
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --case)
      case_name="${2:-}"
      shift 2
      ;;
    --platform)
      platform="${2:-}"
      shift 2
      ;;
    --client-version)
      client_version="${2:-}"
      shift 2
      ;;
    --package)
      package_type="${2:-}"
      shift 2
      ;;
    --enrollment)
      enrollment="${2:-}"
      shift 2
      ;;
    --server-config)
      server_config="${2:-}"
      shift 2
      ;;
    --access)
      access="${2:-}"
      shift 2
      ;;
    --wan)
      wan="${2:-}"
      shift 2
      ;;
    --nat-relay)
      nat_relay="${2:-}"
      shift 2
      ;;
    --update)
      update="${2:-}"
      shift 2
      ;;
    --rollback)
      rollback="${2:-}"
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

if [ -z "$case_name" ] || [ -z "$platform" ] || [ -z "$client_version" ]; then
  usage >&2
  exit 2
fi

if ! valid_case_name "$case_name"; then
  printf 'invalid case name: use 1-64 safe identifier characters only\n' >&2
  exit 2
fi
if ! valid_client_version "$client_version"; then
  printf 'invalid client version: use an official semantic version only\n' >&2
  exit 2
fi
if ! valid_package_type "$package_type"; then
  printf 'invalid package type: use an allowed package label\n' >&2
  exit 2
fi

case "$platform" in
  windows|linux|macos|android|ios) ;;
  *)
    printf 'invalid platform: %s\n' "$platform" >&2
    exit 2
    ;;
esac

for status in "$enrollment" "$server_config" "$access" "$wan" "$nat_relay" "$update" "$rollback"; do
  if ! valid_status "$status"; then
    printf 'invalid status: %s\n' "$status" >&2
    exit 2
  fi
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
mkdir -p local/research/manual
safe_case="$case_name"
stamp="$(date -u +%Y%m%d-%H%M%S)"
target="local/research/manual/validation-${stamp}-${safe_case}-${platform}.md"

cat >"$target" <<EOF
# Validation Evidence Record

Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Case: ${case_name}
Platform: ${platform}
Official client version: ${client_version}
Package type: ${package_type}
Tester:
Commit SHA:

## Status Summary

| Evidence area | Validation IDs | Status |
|---|---|---|
| Enrollment and check-in | E-001 through E-005, E-008 | ${enrollment} |
| Server configuration readback | S-005, S-010, D-001 through D-015 | ${server_config} |
| Dashboard access and RBAC | C-001, C-009, SEC-003, SEC-004, SEC-007 | ${access} |
| WAN path | S-011, R-003 | ${wan} |
| NAT and relay fallback | S-012, R-003 | ${nat_relay} |
| Official update persistence | D-010, S-009 | ${update} |
| Rollback and fallback workflow | CUT-004, CUT-006 | ${rollback} |

## Evidence References

Record paths to raw artifacts under ignored local/research/ only. Do not paste tokens,
keys, passwords, full endpoint IDs, hostnames, addresses, or command output here.

- Enrollment/check-in artifact:
- Server configuration/fingerprint artifact:
- Dashboard RBAC artifact:
- WAN/NAT/relay probe and connection artifacts:
- Update artifact (installer version/checksum and readback):
- Rollback artifact (last-known-good reference and restore transcript):

## Access Boundary

OpenDesk evidence covers dashboard/API authentication and RBAC only. It does not
claim to enforce or deny RustDesk sessions. Any RustDesk connection result records
transport behavior for the tested endpoint and client, not access-control enforcement.

## Review Checklist

- [ ] Evidence artifacts are stored under ignored local/research/.
- [ ] Official client source and signature/checksum are recorded where applicable.
- [ ] Server values are represented only by redacted labels and key fingerprint.
- [ ] Enrollment token was scoped, expired/revoked, and absent from artifacts.
- [ ] LAN, WAN, NAT, and relay outcomes are separated and topology-neutral in summaries.
- [ ] User roles and dashboard/API allow/deny results are recorded.
- [ ] Update persistence was checked before declaring pass.
- [ ] Rollback was exercised with the last known-good procedure.
- [ ] Final status is reviewed by an independent operator.

Final status: not-run
Follow-up:
EOF

printf '%s\n' "$target"
