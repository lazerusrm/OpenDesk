#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/research-macos-client-record.sh --rustdesk PATH [--config CONFIG] [--deploy-token TOKEN] [--case NAME]

Creates ignored macOS RustDesk validation evidence under local/research/manual/.
Run on a disposable macOS test endpoint from the repo root.
USAGE
}

rustdesk_path=""
config_string=""
deploy_credential=""
case_name="macos-client"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --rustdesk)
      rustdesk_path="${2:-}"
      shift 2
      ;;
    --config)
      config_string="${2:-}"
      shift 2
      ;;
    --deploy-token)
      deploy_credential="${2:-}"
      shift 2
      ;;
    --case)
      case_name="${2:-}"
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

if [ -z "$rustdesk_path" ]; then
  usage >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mkdir -p local/research/manual
safe_case="$(printf "%s" "$case_name" | tr -cs 'A-Za-z0-9._-' '-' | sed 's/^-//; s/-$//')"
stamp="$(date -u +%Y%m%d-%H%M%S)"
json_path="local/research/manual/macos-client-${stamp}-${safe_case}.jsonl"
md_path="local/research/manual/macos-client-${stamp}-${safe_case}.md"

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'
}

run_step() {
  local name="$1"
  shift
  local output exit_code
  set +e
  output="$("$@" 2>&1)"
  exit_code="$?"
  set -e
  printf '{"name":%s,"exit_code":%s,"output":%s}\n' \
    "$(printf "%s" "$name" | json_escape)" \
    "$exit_code" \
    "$(printf "%s" "$output" | json_escape)" >>"$json_path"
}

: >"$json_path"
run_step "sw-version" sw_vers
run_step "rustdesk-version" "$rustdesk_path" --version
run_step "get-id-before-config" "$rustdesk_path" --get-id

if [ -n "$config_string" ]; then
  run_step "apply-config" "$rustdesk_path" --config "$config_string"
  run_step "get-id-after-config" "$rustdesk_path" --get-id
  run_step "read-id-server" "$rustdesk_path" --option custom-rendezvous-server
  run_step "read-relay-server" "$rustdesk_path" --option relay-server
  run_step "read-api-server" "$rustdesk_path" --option api-server
  run_step "read-key" "$rustdesk_path" --option key
fi

if [ -n "$deploy_credential" ]; then
  run_step "deploy" "$rustdesk_path" --deploy --token "$deploy_credential"
fi

cat >"$md_path" <<EOF
# macOS Client Validation Record

Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Case: $case_name
RustDesk path: $rustdesk_path
Raw artifact: $json_path
Final status: not-reviewed

## Review Checklist

- [ ] DMG source and checksum/signature were verified separately.
- [ ] Screen/input permissions are documented.
- [ ] Config values are redacted before public summary.
- [ ] Service/root and user context behavior is reviewed.
- [ ] Restart persistence is reviewed.
- [ ] Reinstall/upgrade persistence is reviewed if tested.
- [ ] Deploy endpoint behavior is reviewed if tested.
EOF

printf '%s\n' "$md_path"
