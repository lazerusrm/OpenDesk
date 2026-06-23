#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/research-network-probe.sh --host HOST [--ports PORTS] [--context NAME]

Writes DNS/TCP probe evidence under ignored local/research/manual/.
PORTS defaults to: 80,21114,21115,21116,21117,21118,21119
USAGE
}

host=""
ports="80,21114,21115,21116,21117,21118,21119"
context="unspecified"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --host)
      host="${2:-}"
      shift 2
      ;;
    --ports)
      ports="${2:-}"
      shift 2
      ;;
    --context)
      context="${2:-}"
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

if [ -z "$host" ]; then
  usage >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mkdir -p local/research/manual
safe_context="$(printf "%s" "$context" | tr -cs 'A-Za-z0-9._-' '-' | sed 's/^-//; s/-$//')"
stamp="$(date -u +%Y%m%d-%H%M%S)"
target="local/research/manual/network-${stamp}-${safe_context}.md"

{
  printf '# Network Validation Record\n\n'
  printf 'Date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'Tester:\n'
  printf 'Client network context: %s\n' "$context"
  printf 'DNS result:\n'
  getent ahosts "$host" 2>&1 | sed 's/^/  /' || true
  printf 'TCP port result:\n'
  IFS=',' read -r -a port_list <<<"$ports"
  for port in "${port_list[@]}"; do
    if timeout 3 bash -lc "</dev/tcp/$host/$port" 2>/dev/null; then
      printf '  tcp_%s=open\n' "$port"
    else
      printf '  tcp_%s=closed_or_filtered\n' "$port"
    fi
  done
  printf 'UDP result: not-tested\n'
  printf 'Direct connection result: not-tested\n'
  printf 'Relay connection result: not-tested\n'
  printf 'NAT/hairpin finding: not-tested\n'
  printf 'Artifacts: %s\n' "$target"
  printf 'Final status: not-run\n'
  printf 'Follow-up:\n'
  printf '\n## Redaction Check\n\n'
  printf -- '- [ ] Public docs use only topology-neutral conclusions.\n'
  printf -- '- [ ] Raw hostnames/IPs remain under ignored local/research/.\n'
} >"$target"

printf '%s\n' "$target"
