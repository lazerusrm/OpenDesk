#!/usr/bin/env bash
set -euo pipefail

# Run inside the dev LXC container after OpenDesk is listening.
BASE_URL="${1:-http://127.0.0.1:18080}"
TOKEN_VALUE="${2:?enrollment token value required}"
DEPLOY_ROOT="${OPENDESK_DEPLOY_ROOT:-/opt/opendesk-dev}"

echo "dev-lxc-validate: host=$(hostname)"
echo "dev-lxc-validate: deploy_root=${DEPLOY_ROOT}"
echo "dev-lxc-validate: base_url=${BASE_URL}"

curl -fsS "${BASE_URL}/health" | grep -q '^ok$'
echo "dev-lxc-validate: health ok"

response="$(curl -fsS -o /dev/null -w '%{http_code}' -X POST "${BASE_URL}/api/enrollments/check-in" \
  -H 'Content-Type: application/json' \
  -d "{\"enrollment_token\":\"${TOKEN_VALUE}\",\"rustdesk_id\":\"999000111\",\"hostname\":\"$(hostname)\",\"os_family\":\"linux\",\"architecture\":\"$(uname -m)\",\"rustdesk_version\":\"1.4.8\"}")"

if [ "$response" != "204" ]; then
  echo "dev-lxc-validate: unexpected check-in status ${response}" >&2
  exit 1
fi
echo "dev-lxc-validate: enrollment check-in status=${response}"

if command -v rustdesk >/dev/null 2>&1; then
  echo "dev-lxc-validate: applying rustdesk server options"
  rustdesk --option custom-rendezvous-server rd.example.com
  rustdesk --option relay-server rd.example.com
  rustdesk_id="$(rustdesk --get-id 2>/dev/null || true)"
  echo "dev-lxc-validate: rustdesk_id=${rustdesk_id:-unknown}"
  rendezvous="$(rustdesk --get-option custom-rendezvous-server 2>/dev/null || true)"
  echo "dev-lxc-validate: rustdesk_rendezvous=${rendezvous:-unset}"
else
  echo "dev-lxc-validate: rustdesk client not installed; skipping script apply" >&2
  exit 1
fi

echo "dev-lxc-validate: completed on $(hostname)"