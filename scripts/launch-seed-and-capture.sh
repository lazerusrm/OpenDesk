#!/usr/bin/env bash
set -euo pipefail

SCRATCH="${1:?scratch directory}"
RUN_ID="${2:?run id 1 or 2}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT=$((18080 + RUN_ID))
DATA_DIR="$SCRATCH/launch-data-$RUN_ID"
COOKIE_JAR="$SCRATCH/cookies-$RUN_ID.txt"
BASE="http://127.0.0.1:$PORT"
BINARY="$REPO/target/debug/opendesk"

pgrep -f "$REPO/target/debug/opendesk" | xargs -r kill 2>/dev/null || true
sleep 0.5

rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR"
: >"$COOKIE_JAR"

cd "$REPO"
cargo build --quiet

OPENDESK_LISTEN_ADDR="127.0.0.1:$PORT" \
OPENDESK_DATA_DIR="$DATA_DIR" \
OPENDESK_PUBLIC_BASE_URL="$BASE" \
OPENDESK_BOOTSTRAP_ADMIN_PASSWORD=test-pass \
"$BINARY" 2>"$SCRATCH/launch-$RUN_ID-server.log" &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

LOGIN_STATUS="000"
for _ in $(seq 1 60); do
  if curl -sf "$BASE/health" >/dev/null 2>&1; then
    LOGIN_STATUS=$(curl -s -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
      -X POST "$BASE/login" \
      -d "username=admin&password=test-pass" \
      -o /dev/null -w "%{http_code}")
    if [ "$LOGIN_STATUS" = "303" ]; then
      break
    fi
  fi
  sleep 0.5
done
echo "login_http_status=$LOGIN_STATUS" | tee "$SCRATCH/launch-$RUN_ID-login.log"
if [ "$LOGIN_STATUS" != "303" ]; then
  echo "login failed; server log:" >&2
  cat "$SCRATCH/launch-$RUN_ID-server.log" >&2 || true
  exit 1
fi

curl -sf "$BASE/health" >"$SCRATCH/launch-$RUN_ID-health.txt"

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/settings/server-config" \
  -d "id_server=rd.example.com&relay_server=rd.example.com&public_key=launch-test-public-key" \
  -o /dev/null

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/sites" \
  -d "name=Main+Lab" \
  -o /dev/null

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/tags" \
  -d "name=Production" \
  -o /dev/null

read -r SITE_UUID TAG_UUID <<EOF
$(python3 -c "
import sqlite3, os
db = os.path.join('${DATA_DIR}', 'opendesk.sqlite')
con = sqlite3.connect(db)
site = con.execute(\"SELECT site_uuid FROM sites WHERE name='Main Lab' LIMIT 1\").fetchone()[0]
tag = con.execute(\"SELECT tag_uuid FROM tags WHERE name='Production' LIMIT 1\").fetchone()[0]
print(site, tag)
")
EOF

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/devices" \
  -d "alias=Tagged+Workstation&rustdesk_id=123456789&notes=Operator+runbook+reference&site_uuid=${SITE_UUID}&tag_uuids=${TAG_UUID}" \
  -o /dev/null

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/devices" \
  -d "alias=Archived+Workstation" \
  -o /dev/null

ARCHIVED_UUID=$(python3 -c "
import sqlite3, os
db = os.path.join('${DATA_DIR}', 'opendesk.sqlite')
con = sqlite3.connect(db)
print(con.execute(\"SELECT device_uuid FROM devices WHERE alias='Archived Workstation' LIMIT 1\").fetchone()[0])
")

curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -X POST "$BASE/devices/${ARCHIVED_UUID}/archive" \
  -o /dev/null

curl -sf -b "$COOKIE_JAR" "$BASE/devices" >"$SCRATCH/launch-$RUN_ID-devices.html"
curl -sf -b "$COOKIE_JAR" "$BASE/devices/export.csv" >"$SCRATCH/launch-$RUN_ID-devices.csv"
curl -sf -b "$COOKIE_JAR" "$BASE/backup/export.json" >"$SCRATCH/launch-$RUN_ID-backup.json"
curl -sf -b "$COOKIE_JAR" "$BASE/tags" >"$SCRATCH/launch-$RUN_ID-tags.html"
curl -sf -b "$COOKIE_JAR" "$BASE/deployment" >"$SCRATCH/launch-$RUN_ID-deployment.html"
curl -sf -b "$COOKIE_JAR" "$BASE/status" >"$SCRATCH/launch-$RUN_ID-status.html"

{
  echo "launch-$RUN_ID health=$(cat "$SCRATCH/launch-$RUN_ID-health.txt")"
  echo "launch-$RUN_ID site_uuid=$SITE_UUID tag_uuid=$TAG_UUID archived_uuid=$ARCHIVED_UUID"
  grep -E 'Tagged Workstation|Main Lab|Production|Operator runbook|Copy default|Copy explicit|data-copy-text="123456789@rd.example.com:21117\?key=launch-test-public-key"' "$SCRATCH/launch-$RUN_ID-devices.html" || true
  if ! grep -q 'Copy explicit' "$SCRATCH/launch-$RUN_ID-devices.html"; then
    echo "launch-$RUN_ID explicit_helper_button=missing"
    exit 1
  fi
  if ! grep -q 'data-copy-text="123456789@rd.example.com:21117?key=launch-test-public-key"' "$SCRATCH/launch-$RUN_ID-devices.html"; then
    echo "launch-$RUN_ID explicit_helper_attr=missing"
    exit 1
  fi
  echo "launch-$RUN_ID explicit_helper=ok"
  grep -E 'macOS shell script|rustdesk-host=rd.example.com|Official RustDesk clients' "$SCRATCH/launch-$RUN_ID-deployment.html" || true
  grep -E 'Public key fingerprint|tcp:rd.example.com:21116|tcp:rd.example.com:21117|dns:rd.example.com' "$SCRATCH/launch-$RUN_ID-status.html" || true
  head -2 "$SCRATCH/launch-$RUN_ID-devices.csv" | tee -a "$SCRATCH/launch-$RUN_ID-summary.log" || true
  grep -E 'device_uuid,alias|Tagged Workstation|123456789' "$SCRATCH/launch-$RUN_ID-devices.csv" || true
  grep -E '"schema_version": 1|"excludes_sessions": true|Tagged Workstation' "$SCRATCH/launch-$RUN_ID-backup.json" || true
  if grep -q 'Archived Workstation' "$SCRATCH/launch-$RUN_ID-devices.html"; then
    echo "launch-$RUN_ID archived_hidden=no"
    exit 1
  fi
  echo "launch-$RUN_ID archived_hidden=yes"
  grep -c '<tr>' "$SCRATCH/launch-$RUN_ID-devices.html" | awk '{print "launch-'"$RUN_ID"' table_rows="$1}'
} | tee "$SCRATCH/launch-$RUN_ID-summary.log"

kill "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
trap - EXIT