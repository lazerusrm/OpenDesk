#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="${OPENDESK_VALIDATION_SCRATCH:?set OPENDESK_VALIDATION_SCRATCH to a local scratch directory}"
proxmox_host="${OPENDESK_PROXMOX_HOST:?set OPENDESK_PROXMOX_HOST to the Proxmox host}"
proxmox_key="${OPENDESK_PROXMOX_KEY:-$HOME/.ssh/proxmox_lan}"
vmid="${OPENDESK_DEV_VMID:?set OPENDESK_DEV_VMID to the dev container VMID}"
dev_admin_password="${OPENDESK_DEV_ADMIN_PASSWORD:?set OPENDESK_DEV_ADMIN_PASSWORD for dev validation}"
log_file="${scratch}/dev-lxc-remote-transcript.log"

mkdir -p "$scratch"
: >"$log_file"
rm -f "${scratch}/dev-lxc-validation.log"

log() {
  printf '%s\n' "$*" | tee -a "$log_file"
}

log "=== OpenDesk dev LXC validation transcript ==="
log "timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
log "proxmox_host=${proxmox_host}"
log "vmid=${vmid}"

cd "$repo_root"
cargo build --release 2>&1 | tee -a "$log_file" | tail -3

tar czf "${scratch}/opendesk-bundle.tar.gz" \
  target/release/opendesk migrations templates static

log "=== pct push bundle to VMID ${vmid} ==="
scp -i "$proxmox_key" "${scratch}/opendesk-bundle.tar.gz" "root@${proxmox_host}:/tmp/opendesk-bundle.tar.gz"
ssh -i "$proxmox_key" "root@${proxmox_host}" "pct push ${vmid} /tmp/opendesk-bundle.tar.gz /tmp/opendesk-bundle.tar.gz"

log "=== pct exec ${vmid}: deploy, fetch generated script, execute ==="
ssh -i "$proxmox_key" "root@${proxmox_host}" "pct exec ${vmid} -- bash -s" <<REMOTE | tee -a "$log_file"
set -euo pipefail
echo "remote: host=\$(hostname)"
echo "remote: pwd=\$(pwd)"
rm -rf /opt/opendesk-dev/data
mkdir -p /opt/opendesk-dev/data
cd /opt/opendesk-dev
tar xzf /tmp/opendesk-bundle.tar.gz
chmod +x target/release/opendesk
pkill -x opendesk 2>/dev/null || true
OPENDESK_LISTEN_ADDR=0.0.0.0:18080 \\
OPENDESK_DATA_DIR=/opt/opendesk-dev/data \\
OPENDESK_PUBLIC_BASE_URL=http://127.0.0.1:18080 \\
OPENDESK_BOOTSTRAP_ADMIN_PASSWORD='${dev_admin_password}' \\
nohup ./target/release/opendesk > /tmp/opendesk.log 2>&1 &
sleep 2
COOKIE=/tmp/opendesk-cookies.txt
rm -f "\$COOKIE"
curl -fsS -c "\$COOKIE" -b "\$COOKIE" -X POST http://127.0.0.1:18080/login \\
  -H 'Content-Type: application/x-www-form-urlencoded' \\
  --data "username=admin&password=${dev_admin_password}" -o /dev/null
html="\$(curl -fsS -c "\$COOKIE" -b "\$COOKIE" -X POST http://127.0.0.1:18080/enrollment-tokens \\
  -H 'Content-Type: application/x-www-form-urlencoded' \\
  --data 'label=dev-lxc-validation')"
TOKEN_VALUE="\$(printf '%s' "\$html" | sed -n 's/.*<code>\([^<]*\)<\/code>.*/\1/p' | head -1)"
if [ -z "\$TOKEN_VALUE" ]; then
  echo "remote: failed to extract enrollment token" >&2
  exit 1
fi
echo "remote: enrollment_token_created=yes"
curl -fsS -c "\$COOKIE" -b "\$COOKIE" \\
  "http://127.0.0.1:18080/deployment/linux.sh?enrollment_token_value=\${TOKEN_VALUE}" \\
  -o /tmp/opendesk-deploy.sh
chmod +x /tmp/opendesk-deploy.sh
head -5 /tmp/opendesk-deploy.sh
echo "remote: executing generated deployment script"
bash /tmp/opendesk-deploy.sh
if command -v rustdesk >/dev/null 2>&1; then
  rendezvous="\$(rustdesk --get-option custom-rendezvous-server 2>/dev/null || true)"
  rustdesk_id="\$(rustdesk --get-id 2>/dev/null || true)"
  echo "remote: rustdesk_rendezvous=\${rendezvous:-unset}"
  echo "remote: rustdesk_id=\${rustdesk_id:-unknown}"
else
  echo "remote: rustdesk client not installed" >&2
  exit 1
fi
echo "remote: generated script execution completed on \$(hostname)"
REMOTE

log "=== dev LXC validation complete; transcript=${log_file} ==="