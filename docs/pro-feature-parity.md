# Pro Feature Parity Map

This document tracks RustDesk Server Pro-style capabilities and how OpenDesk intends to cover them without depending on RustDesk Pro infrastructure.

Status categories:

- `Core`: required for full replacement.
- `Stage 2`: required if current production workflow depends on it, otherwise next wave.
- `Research`: possible, but requires validation against official clients or deeper integration.
- `Deferred`: out of scope until there is a concrete business need.

## Feature Map

| Capability | OpenDesk Approach | Status | Notes |
|---|---|---:|---|
| Self-hosted ID server | Use OSS `hbbs` | Core | RustDesk owns the protocol behavior. OpenDesk monitors/configures it. |
| Self-hosted relay server | Use OSS `hbbr` | Core | Same. |
| Web admin console | Build OpenDesk web UI | Core | Core product surface. |
| Device inventory | OpenDesk database | Core | Manual CRUD plus self-registration before cutover. |
| Address book | OpenDesk device list/bookmarks | Core | External web address book first; native RustDesk address book remains research. |
| Tags/groups/sites | OpenDesk metadata | Core | Used for search, filtering, ownership, and deployment targeting. |
| Device notes | OpenDesk metadata | Core | Public-safe examples only in docs. |
| Client config distribution | Generated scripts/download instructions | Core | Avoid executable renaming as the primary path. |
| Download configured client | Serve generated per-OS install/config flows | Core | May include cached official binaries later. |
| Filename-based client config | Generate `rustdesk-host=...` filename | Stage 2 | Fallback/convenience path; validate per OS/version. |
| Endpoint self-registration | OpenDesk enrollment API/script | Core | Enrollment tokens scoped and revocable. |
| `/api/devices/deploy` compatibility | Implement RustDesk-shaped endpoint | Research | Only if official clients can use it without Pro friction. |
| Audit log | OpenDesk audit events | Core | Covers admin actions and endpoint enrollment, not full RustDesk session internals. |
| Session audit | Client/server log ingestion if available | Research | Requires reliable source of session events. |
| Access control | OpenDesk UI/API roles | Core | Controls dashboard visibility/actions. Does not enforce RustDesk session auth by itself. |
| RustDesk session ACLs | Endpoint/network/client integration | Research | Must not be claimed until actually enforced. |
| Central settings/policies | Generated config + endpoint registration service | Core | Use scripts/service first; native client policy later if required. |
| Disable public server fallback | Scripted config where supported | Core | Hard enforcement may require client policy/fork. |
| Managed unattended passwords | External secret manager integration | Research | ADR-008 applies if this becomes required. |
| SSO/OIDC | Reverse proxy or app-native OIDC | Stage 2 | Required before cutover only if used today. |
| LDAP | External IdP integration | Deferred | Prefer OIDC via Authentik/Authelia/etc. |
| 2FA | Reverse proxy/IdP first | Stage 2 | Required before cutover only if used today. |
| Passkeys | App-native WebAuthn/passkey support | Stage 2 | Desired as soft opt-in OpenDesk auth hardening, especially phone passkeys. Not RustDesk session enforcement. |
| Custom client builder | Generated scripts/wrapper | Research | Full custom client builds are deferred. |
| Branding | OpenDesk web UI branding | Core | RustDesk client branding deferred. |
| Native RustDesk address book | Client fork or compatible API | Research | Required only if the external OpenDesk address book does not satisfy production workflow. |
| Browser/web remote client | Do not build initially | Deferred | Large separate project. |
| Mobile app operator workflow | Generated manual/QR config instructions | Core | Android and iOS RustDesk apps must work for operators before cutover. Official app distribution stays RustDesk-owned. |
| Backups | OpenDesk backup/restore | Core | Include database/config, exclude runtime junk/secrets where possible. |
| Health checks | DNS/port/service/key fingerprint checks | Core | Start with external reachability checks. |

## Production Usage Inventory

Inventory recorded 2026-06-29 from read-only production-style database inspection, owner decisions in `docs/research/owner-decisions.md`, and dev LXC Linux package validation. No unresolved Used Today values remain.

| Pro Area | Used Today | Replacement Path | Validation IDs | Evidence | Blocker |
|---|---|---|---|---|---|
| Users and admin accounts | yes | OpenDesk admin login; multi-user RBAC Phase 5 | C-001, C-009 | Inspected Pro users table populated; OpenDesk auth implemented | RBAC matrix at cutover |
| Device groups / assignments | yes | OpenDesk sites, tags, device metadata | C-003, C-004, C-005 | Inspected device-user assignments | none for research closure |
| Personal address books | yes | OpenDesk device list, search, RustDesk ID copy (main); connection helpers (PR #14) | C-002, C-003, CUT-003 | Inspected address-book entries linked to devices | Pilot operator workflow |
| Native RustDesk address book | no | Equivalent: OpenDesk web address book | CUT-003 | Owner decision: equivalent workflow | Pilot confirmation |
| Managed/passwordless address-book access | yes | Equivalent: documented copy/helper workflow (PR #14) + operator-managed credentials; ADR-008 if upgraded | IR-003, CUT-003 | Hashed secrets in Pro entries; OpenDesk rejects plaintext storage | Pilot operator workflow |
| Strategies/policies | yes | Generated config + enrollment; policy UI Stage 2 | D-001, CUT-003 | Active strategy config rows in inspected DB | Pilot policy parity |
| Windows custom clients | yes | Generated Windows PowerShell script on `/deployment` | D-001, D-002, CUT-003 | Windows-only custom clients in inspected DB | Pilot Windows install |
| Sessions / connection history | yes | OpenDesk device list + optional Tier 2 log ingestion | C-007, RS-007 | Connection audit rows populated | Tier 2 optional |
| Console / admin audit | yes | OpenDesk Tier 1 audit; optional RustDesk ingestion | C-007, PR-009 | Console audit rows with recent activity | Ingestion labels if Tier 2 |
| Relay / ID server management | yes | OSS `hbbs`/`hbbr` unchanged; plain `/health` on main; `/status` dashboard PR #14 | C-008, S-001 through S-005 | Service ports and logs validated | WAN pilot S-004 |
| 2FA | no | Retired for parity; optional IdP hardening | SEC-009 | No users with 2FA in inspected DB | none |
| SSO / third-party auth | no | Retired for parity; optional reverse-proxy OIDC Stage 2 | SEC-001 | No third-party auth rows in inspected DB | none |
| Passkeys | no | Optional OpenDesk login hardening | SEC-009 | Owner preference only | Implementation Phase 5+ |
| API integrations / deploy endpoint | no | Retired for cutover; generated scripts primary; R-006 defers adapter | E-006, E-007 | Linux deploy contract validated; adapter deferred | Reopen only if pilot requires |
| Mobile operator workflow | yes | Official RustDesk apps + OpenDesk config instructions | D-011, D-012, CUT-003 | Official mobile docs; accepted-exception until pilot | Pilot Android/iOS |

## Replacement Acceptance

OpenDesk can be considered a Pro replacement for the owner when:

- Daily device lookup works from the OpenDesk dashboard.
- New Windows and Linux devices can be installed/configured from OpenDesk without manual RustDesk server entry.
- Devices can self-register or be added with minimal manual metadata.
- Operators can find and connect to devices reliably.
- Server health and backups are visible.
- Existing RustDesk remote sessions continue to use official clients and OSS relay/ID services.
- No production secrets or site-specific values are committed.
- Every Pro feature currently used in production is represented in this parity map with a passing implementation, a validated equivalent workflow, or an explicit decision that it is not needed.
- Cutover remains blocked until the validation matrix passes for required operating systems and workflows.

## Acceptable Gaps Before Cutover

These are acceptable before cutover only if the owner confirms they are not required by the current production workflow:

- Native address book inside the RustDesk app.
- Full RustDesk session-level ACL enforcement.
- Managed password injection.
- Browser remote desktop.
- Custom signed desktop/mobile clients.
- Drop-in RustDesk Pro API compatibility.
