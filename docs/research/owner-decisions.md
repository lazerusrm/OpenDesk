# Owner Decision Worksheet

This worksheet turns the remaining research gaps into explicit owner decisions. Private supporting evidence can live under ignored `local/research/`.

## Decision States

- `required`: OpenDesk must replace this before cutover.
- `equivalent`: A different OpenDesk workflow is acceptable.
- `retired`: The workflow is not needed after cutover.
- `unresolved`: Cutover blocker until owner records required, equivalent, retired, or optional.

## Pro Usage Decisions

| Area | Current Evidence | Decision | Replacement Path / Validation |
|---|---|---|---|
| Windows custom clients | Populated and Windows-only in inspected database. | Required | Generated Windows install/config script on `/deployment`; pilot validation D-001, D-002, CUT-003. |
| Strategies/policies | Strategy rows include config options. | Required | OpenDesk generated config + enrollment service; policy UI deferred Stage 2; validation D-001, CUT-003. |
| Personal address books | Mostly personal address books with linked devices. | Required | OpenDesk device list, search, sites/tags, RustDesk ID copy; connection helpers in PR #14; validation C-002, C-003, CUT-003. |
| Native RustDesk address book | Native app parity is not yet proven necessary. | Equivalent | OpenDesk web address book and mobile browser/operator workflow; validation CUT-003. |
| Managed/passwordless address-book access | Hashed secret material exists in address-book entries. | Equivalent | Documented copy/helper workflow (PR #14) and operator-managed endpoint credentials; no plaintext unattended passwords in OpenDesk; ADR-008 if managed-password parity is later required. |
| Device/user assignments | Device records are assigned to users. | Required | OpenDesk device metadata and site/tag ownership; validation C-003, C-004. |
| Control roles | Rows exist, but inspected mappings were empty. | Equivalent | OpenDesk RBAC model (Phase 5); validation C-009, SEC-008 at cutover. |
| 2FA | No current user rows had 2FA enabled. | Retired | Optional OpenDesk/IdP hardening only; not a Pro parity blocker. |
| Third-party auth | No inspected third-party auth rows. | Retired | Optional reverse-proxy OIDC/SSO Stage 2; not a current production dependency. |
| Passkeys | Desired as soft opt-in OpenDesk auth hardening, especially mobile phone passkeys. | Optional | OpenDesk WebAuthn hardening only; validation SEC-009 when implemented. |
| Audit logs | Connection and console audit tables are populated. | Required | OpenDesk first-party audit (Tier 1); optional RustDesk log ingestion (Tier 2); validation C-007, PR-009, RS-007. |
| Relay management | Relay config exists and services are active. | Required | Plain `/health` on `main`; `/status` dashboard in PR #14; server config page; validation C-008, S-001 through S-005. |

## Access Model Decisions

| Question | Why It Matters | Decision |
|---|---|---|
| Does OpenDesk need to enforce who can start a RustDesk session? | Dashboard RBAC alone does not prevent a direct RustDesk connection if ID/password access still works. | Lookup-only: OpenDesk controls dashboard visibility and copy/config workflows only; no RustDesk session ACL claims. |
| Are endpoint passwords shared outside the dashboard today? | Shared secrets weaken any dashboard-only access model. | Accept with documented risk; operators rotate endpoint credentials per runbook; OpenDesk does not store plaintext unattended passwords. |
| Must operators get passwordless one-click access? | This would require a secret-management design. | Retired for cutover; ADR-008 external secret-manager path if owner upgrades requirement later. |
| Is session audit required or is launch-intent audit enough? | Logs can show useful events, but OpenDesk launch intent is not proof of a completed session. | Tier 1 OpenDesk audit required for admin/enrollment actions; Tier 2 RustDesk log/database ingestion optional Stage 2; launch-intent is not session proof. |

## Passkey Scope

Passkeys are desired as a soft opt-in feature for OpenDesk login, especially mobile phone passkeys. They should protect access to the OpenDesk dashboard/API only.

Passkeys do not replace:

- RustDesk unattended passwords.
- Endpoint password rotation.
- Native RustDesk session authorization.
- Any future client/server session enforcement design.

## Current Recommended Decisions

- Use generated official-client install/config flows instead of Pro custom clients.
- Use OpenDesk web device list/address book first.
- Do not store plaintext unattended passwords.
- Treat session-level enforcement as out of scope until a real enforcement mechanism exists.
- Require mobile RustDesk app operator workflows before cutover; treat managed mobile endpoints as separate future scope unless later required.
- Support passkeys as optional OpenDesk login hardening, separate from RustDesk unattended/session passwords.

## Signoff

Research closure signoff date: 2026-06-29.

Decisions above are recorded from read-only production-style database inspection, RustDesk `1.4.8` Debian package validation in the dev LXC, official RustDesk client documentation, local upstream source inspection, and current OpenDesk implementation evidence. Platform-specific gaps closed via accepted exceptions documented in `docs/research-status.md` and `docs/research-closure-packets.md`, with remaining proof assigned to pilot validation (`CUT-003`) before production cutover.

Cutover remains blocked until pilot validation passes for required workflows; research rows R-001 through R-009 are accepted for planning purposes.