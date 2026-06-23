#!/usr/bin/env bash
set -euo pipefail

# Run on the dev LXC after OpenDesk is listening locally.
BASE_URL="${1:-http://127.0.0.1:18080}"
TOKEN_VALUE="${2:?enrollment token value required}"

curl -fsS "${BASE_URL}/health" | grep -q '^ok$'

response="$(curl -fsS -o /dev/null -w '%{http_code}' -X POST "${BASE_URL}/api/enrollments/check-in" \
  -H 'Content-Type: application/json' \
  -d "{\"enrollment_token\":\"${TOKEN_VALUE}\",\"rustdesk_id\":\"999000111\",\"hostname\":\"opendesk-dev\",\"os_family\":\"linux\",\"architecture\":\"$(uname -m)\",\"rustdesk_version\":\"1.4.8\"}")"

if [ "$response" != "204" ]; then
  echo "dev-lxc-validate: unexpected check-in status ${response}" >&2
  exit 1
fi

if command -v rustdesk >/dev/null 2>&1; then
  rustdesk --option custom-rendezvous-server rd.example.com >/dev/null
fi

echo "dev-lxc-validate: enrollment check-in succeeded against ${BASE_URL}"