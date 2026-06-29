# Feature Checklist

Status legend:

- `[ ]` not started
- `[~]` design needed
- `[x]` complete

## Phase 0: Discovery

- [~] Snapshot current RustDesk LXC.
- [x] Record current RustDesk server type: OSS vs Pro.
- [x] Record current Docker Compose/systemd files.
- [x] Record current server key/public key location.
- [~] Record exposed ports and router/firewall rules.
- [x] Provision disposable dev LXC for validation.
- [ ] Export current Pro address book/device list if available.
- [~] Identify which Pro features are actually used today.
- [x] Clone upstream RustDesk client locally as ignored reference.
- [x] Clone upstream RustDesk server locally as ignored reference.
- [x] Record upstream commit hashes in docs.
- [x] Confirm upstream server role: OSS `hbbs`/`hbbr` rendezvous/relay.
- [x] Identify current client custom-server filename parser.
- [x] Identify current client `/api/devices/deploy` compatibility hook.
- [~] Complete R-001 client configuration behavior research.
- [~] Complete R-002 official client deployment mechanics research.
- [~] Complete R-003 current RustDesk Pro usage inventory.
- [~] Complete R-004 address book/password model research.
- [~] Complete R-005 access control reality research.
- [~] Complete R-006 deployment endpoint compatibility research.
- [~] Complete R-007 session/audit log sources research.
- [~] Complete R-008 mobile workflow research.
- [~] Complete R-009 relay/NAT/LAN/DNS behavior research.
- [x] Complete R-010 legal/license posture research.

## Phase 1: Core Control Plane

- [x] Admin login.
- [x] Device CRUD.
- [x] Device archive/unarchive.
- [x] Device search by alias, hostname, RustDesk ID, site, tag.
- [x] Site/location management.
- [x] Tags.
- [x] Notes.
- [x] RustDesk ID copy button.
- [ ] Connection helper action.
- [x] CSV export.
- [x] JSON backup export.
- [x] JSON backup restore.

## Phase 2: Client Configuration and Downloads

- [x] Store canonical RustDesk server config:
  - ID server: `rd.example.com`
  - Relay server: `rd.example.com`
  - API server: blank for OSS path
  - Public key: imported from current server
- [ ] Generate RustDesk import string if supported by current client.
- [ ] Generate filename-based custom server download name as fallback.
- [x] Generate Windows PowerShell installer/config script.
- [x] Generate Linux installer/config script.
- [ ] Generate macOS installer/config script if required.
- [ ] Provide official-client download links or cached installer packages.
- [ ] Provide a single frontend page per OS with install command/download.
- [ ] Avoid executable renaming as the main workflow.
- [ ] Version generated scripts.
- [ ] Audit generated downloads/scripts.
- [ ] Validate whether `/api/devices/deploy` can be used by official clients without Pro.
- [ ] Implement `/api/devices/deploy` compatibility only if validation passes.

## Phase 3: Endpoint Self-Registration

- [x] Enrollment token model.
- [x] Enrollment token creation/rotation/revocation.
- [x] Endpoint registration API.
- [ ] Windows self-registration script.
- [x] Linux self-registration script.
- [ ] macOS self-registration script if required.
- [x] Duplicate detection by RustDesk ID and hostname.
- [x] Last check-in timestamp.
- [ ] Endpoint metadata update.
- [ ] Endpoint registration audit events.

## Phase 4: Health and Operations

- [ ] Check `hbbs` TCP ports.
- [ ] Check `hbbr` TCP ports.
- [ ] Check UDP `21116` reachability where feasible.
- [ ] Check DNS resolution for `rd.example.com`.
- [ ] Check public IP expectation.
- [ ] Show current server public key fingerprint.
- [ ] Backup scheduler.
- [ ] Restore procedure.
- [ ] Log rotation.
- [ ] Upgrade procedure.

## Phase 5: Access and Governance

- [ ] Multi-user admin accounts.
- [ ] Role model: admin, operator, read-only.
- [ ] Device visibility by site/tag.
- [ ] Audit log UI.
- [ ] Export audit log.
- [ ] Optional reverse proxy SSO.
- [ ] Optional OIDC.
- [ ] Optional password vault integration.

## Phase 6: Deferred Native Integration

- [ ] Inspect whether official client supports useful local config files/import behavior.
- [ ] Inspect whether protocol handler launch links are viable on target OSes.
- [ ] Decide whether a light RustDesk client fork is justified.
- [ ] If forked, keep patch set limited to defaults/branding/dashboard integration.
- [ ] Document AGPL obligations before any forked upstream code is committed.
