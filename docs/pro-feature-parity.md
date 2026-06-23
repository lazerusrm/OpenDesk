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
| Mobile app management | Document manual config | Deferred | Official app distribution stays RustDesk-owned. |
| Backups | OpenDesk backup/restore | Core | Include database/config, exclude runtime junk/secrets where possible. |
| Health checks | DNS/port/service/key fingerprint checks | Core | Start with external reachability checks. |

## Production Usage Inventory

Read-only production-style evidence currently proves these populated Pro areas exist: users, groups, device assignments, personal address books, strategies with config options, Windows custom clients, sessions, and audit logs. No current user rows had 2FA enabled in the inspected database, and third-party auth rows were absent.

Current replacement assumptions until owner review:

- Windows generated install/config flow is required because current custom clients are Windows-only.
- Strategy/policy replacement is required because active strategy config exists.
- Native passwordless address-book parity remains undecided because address-book entries contain hashed secret material.
- SSO/third-party auth and 2FA are not currently proven production dependencies, but may still be added as hardening features.
- Passkeys are desired as optional OpenDesk login hardening and should not be treated as a replacement for endpoint password or session enforcement.

Before cutover, the owner must inventory current RustDesk Server Pro usage from operator interviews and any available Pro exports/screenshots. For every capability above, record:

| Field | Required Meaning |
|---|---|
| Used Today | `yes`, `no`, `unknown`, or `retired by owner decision`. |
| Replacement Path | OpenDesk feature, equivalent workflow, or explicit retirement decision. |
| Validation IDs | Validation cases proving the replacement path. |
| Evidence | Link/path to test evidence, pilot notes, export comparison, or owner signoff. |
| Blocker | Remaining gap, or `none`. |

Any `unknown` Used Today value blocks cutover. Any `yes` value needs either passing validation evidence or a signed retirement decision before OpenDesk can be treated as a full Pro replacement.

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
