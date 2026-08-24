#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/fetch-rustdesk-server.sh"
fixture="$root/tests/fixtures/rustdesk-server-release.json"

bash -n "$script"

asset="$("$script" --print-asset)"
case "$asset" in
  rustdesk-server-linux-amd64.zip | rustdesk-server-linux-arm64v8.zip | rustdesk-server-linux-armv7.zip | rustdesk-server-linux-i386.zip) ;;
  *)
    echo "unexpected asset name: $asset" >&2
    exit 1
    ;;
esac

mapfile -t meta < <("$script" --select-json "$fixture")
[[ "${meta[0]}" == "1.1.16" ]] || {
  echo "tag mismatch: ${meta[0]}" >&2
  exit 1
}
[[ "${meta[1]}" == "https://github.com/rustdesk/rustdesk-server/releases/download/1.1.16/${asset}" ]] || {
  echo "url mismatch: ${meta[1]}" >&2
  exit 1
}
[[ "${#meta[2]}" -eq 64 ]] || {
  echo "digest mismatch: ${meta[2]}" >&2
  exit 1
}

if "$script" --no | grep -q "skipped official hbbs/hbbr fetch"; then
  :
else
  echo "expected skip message" >&2
  exit 1
fi

printf 'fetch-rustdesk-server checks passed\n'
