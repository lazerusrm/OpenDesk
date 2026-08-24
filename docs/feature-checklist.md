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
- [x] Connection helper action.
- [x] CSV export.
- [x] JSON backup export.
- [x] JSON backup restore.

## Phase 2: Client Configuration and Downloads

- [x] Store canonical RustDesk server config:
  - ID server: `rd.example.com`
  - Relay server: `rd.example.com`
  - API server: blank for OSS path
  - Public key: imported from current server
- [x] Generate RustDesk import string if supported by current client.
- [x] Generate filename-based custom server download name as fallback.
- [x] Generate Windows PowerShell installer/config script.
- [x] Generate Linux installer/config script.
- [x] Generate macOS installer/config script if required.
- [x] Provide official-client download links or cached installer packages.
- [x] Serve an operator-provisioned signed Windows installer from the deployment page when present.
- [x] Provide a single frontend page per OS with install command/download.
- [x] Avoid executable renaming as the main workflow.
- [ ] Version generated scripts.
- [x] Audit generated downloads/scripts.
- [~] Validate `/api/devices/deploy` on remaining Windows/macOS official clients.
- [x] Implement `/api/devices/deploy` compatibility only if validation passes.

## Phase 3: Endpoint Self-Registration

- [x] Enrollment token model.
- [x] Enrollment token creation/rotation/revocation.
- [x] Enrollment token optional expiry in the dashboard create form.
- [x] Endpoint registration API.
- [x] Windows self-registration script.
- [x] Linux self-registration script.
- [x] macOS self-registration script if required.
- [x] Duplicate detection by RustDesk ID and hostname.
- [x] Last check-in timestamp.
- [x] Endpoint metadata update.
- [x] Endpoint registration audit events.
- [x] Public `/onboard/{token}` install page without recipient login.
- [x] Public `/onboard` six-digit authenticator unlock without recipient login.
- [x] Onboard check-in grants visibility to the issuing operator only.

## Phase 4: Health and Operations

- [x] Check `hbbs` TCP ports.
- [x] Check `hbbr` TCP ports.
- [ ] Check UDP `21116` reachability where feasible.
- [x] Check DNS resolution for `rd.example.com`.
- [ ] Check public IP expectation.
- [x] Show current server public key fingerprint.
- [~] Backup scheduler readiness/configuration (external runner; no in-process scheduler).
- [x] Restore procedure.
- [ ] Log rotation.
- [ ] Upgrade procedure.
- [x] Public self-host install guide for OpenDesk plus OSS `hbbs`/`hbbr`.
- [x] First-run setup wizard when the database has no users (fixed username `admin`).

## Phase 5: Access and Governance

- [x] Multi-user admin accounts.
- [x] Role model: admin, operator, read-only.
- [x] Canonical dashboard/API action allowlist and role predicate.
- [x] Default-deny device visibility predicate for direct/access-group grants.
- [x] Device visibility integration by access group/direct grant.
- [x] Access-group to access-group visibility grants for dashboard/API and official-client data scope.
- [x] Admin sees all devices; unassigned devices are admin-only.
- [x] Address-book share grants in the dashboard for shared books.
- [x] Official-client personal address book auto-create and visible-device sync.
- [x] Official-client accessible-device list includes user and device-group names for local filters.
- [x] Admin-only deletes; operators cannot delete inventory or address-book entries.
- [x] Sign-in attempt throttling and failed-login audit.
- [x] Disable-user revokes dashboard sessions and official-client tokens.
- [x] Audit log UI.
- [x] Export audit log.
- [ ] Optional reverse proxy SSO.
- [ ] Optional OIDC.
- [ ] Optional password vault integration.

## Phase 6: Deferred Native Integration

- [ ] Inspect whether official client supports useful local config files/import behavior.
- [ ] Inspect whether protocol handler launch links are viable on target OSes.
- [ ] Decide whether a light RustDesk client fork is justified.
- [ ] If forked, keep patch set limited to defaults/branding/dashboard integration.
- [ ] Document AGPL obligations before any forked upstream code is committed.
