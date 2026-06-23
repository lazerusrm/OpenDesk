#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: scripts/research-mobile-config-record.sh --case NAME --host HOST --key KEY [--relay RELAY] [--api API]

Creates ignored Android/iOS mobile config validation evidence under local/research/manual/.
Values passed to this script are written only to ignored local evidence files.
USAGE
}

case_name="mobile-config"
host_value=""
key_value=""
relay_value=""
api_value=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --case)
      case_name="${2:-}"
      shift 2
      ;;
    --host)
      host_value="${2:-}"
      shift 2
      ;;
    --key)
      key_value="${2:-}"
      shift 2
      ;;
    --relay)
      relay_value="${2:-}"
      shift 2
      ;;
    --api)
      api_value="${2:-}"
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

if [ -z "$host_value" ] || [ -z "$key_value" ]; then
  usage >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mkdir -p local/research/manual
safe_case="$(printf "%s" "$case_name" | tr -cs 'A-Za-z0-9._-' '-' | sed 's/^-//; s/-$//')"
stamp="$(date -u +%Y%m%d-%H%M%S)"
json_path="local/research/manual/mobile-config-${stamp}-${safe_case}.json"
payload_path="local/research/manual/mobile-config-${stamp}-${safe_case}.payload.txt"
md_path="local/research/manual/mobile-config-${stamp}-${safe_case}.md"

python3 - "$host_value" "$key_value" "$relay_value" "$api_value" "$json_path" "$payload_path" <<'PY'
import json
import sys

host, public_key, relay, api, json_path, payload_path = sys.argv[1:]
config = {"host": host, "key": public_key}
if relay:
    config["relay"] = relay
if api:
    config["api"] = api

payload = "config=" + json.dumps(config, separators=(",", ":"))
with open(json_path, "w", encoding="utf-8") as handle:
    json.dump(
        {
            "config": config,
            "qr_payload": payload,
            "redaction_required": True,
        },
        handle,
        indent=2,
        sort_keys=True,
    )
with open(payload_path, "w", encoding="utf-8") as handle:
    handle.write(payload + "\n")
PY

cat >"$md_path" <<EOF
# Mobile Config Validation Record

Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Case: $case_name
Payload artifact: $payload_path
JSON artifact: $json_path
Final status: not-reviewed

## Android Checklist

- [ ] QR payload imports ID/key settings.
- [ ] Manual ID/relay/key setup works.
- [ ] Mobile operator can connect to a test endpoint.
- [ ] Mobile endpoint control is either validated or explicitly out of scope.

## iOS Checklist

- [ ] iOS operator can connect to a test endpoint if mobile operators are required.
- [ ] No requirement depends on controlling iOS as an endpoint.

## Redaction Check

- [ ] Payload values remain only under ignored local/research/.
- [ ] Public docs summarize only supported/unsupported behavior.
EOF

printf '%s\n' "$md_path"
