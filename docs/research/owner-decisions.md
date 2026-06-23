# Owner Decision Worksheet

This worksheet turns the remaining research gaps into explicit owner decisions. Private supporting evidence can live under ignored `local/research/`.

## Decision States

- `required`: OpenDesk must replace this before cutover.
- `equivalent`: A different OpenDesk workflow is acceptable.
- `retired`: The workflow is not needed after cutover.
- `unknown`: Cutover blocker.

## Pro Usage Decisions

| Area | Current Evidence | Required Decision | Default Until Decided |
|---|---|---|---|
| Windows custom clients | Populated and Windows-only in inspected database. | Required, equivalent, or retired. | Required: generated Windows install/config flow. |
| Strategies/policies | Strategy rows include config options. | Required, equivalent, or retired. | Required: OpenDesk policy model. |
| Personal address books | Mostly personal address books with linked devices. | Required, equivalent, or retired. | Required: OpenDesk address book/device list. |
| Native RustDesk address book | Native app parity is not yet proven necessary. | Required, equivalent, or retired. | Equivalent: OpenDesk web address book, including mobile browser/operator workflow. |
| Managed/passwordless address-book access | Hashed secret material exists in address-book entries. | Required, equivalent, or retired. | Unknown; blocks cutover until owner decides. |
| Device/user assignments | Device records are assigned to users. | Required, equivalent, or retired. | Required: OpenDesk ownership metadata. |
| Control roles | Rows exist, but inspected mappings were empty. | Required, equivalent, or retired. | Equivalent: OpenDesk role model. |
| 2FA | No current user rows had 2FA enabled. | Required hardening or retired. | Retired for parity, optional for hardening. |
| Third-party auth | No inspected third-party auth rows. | Required hardening or retired. | Retired for parity, optional for hardening. |
| Passkeys | Desired as soft opt-in OpenDesk auth hardening, especially mobile phone passkeys. | Optional hardening scope and rollout policy. | Optional; not a Pro parity blocker. |
| Audit logs | Connection and console audit tables are populated. | Required, equivalent, or retired. | Required: OpenDesk audit plus optional ingestion. |
| Relay management | Relay config exists and services are active. | Required, equivalent, or retired. | Required: health/config visibility. |

## Access Model Decisions

| Question | Why It Matters | Required Decision |
|---|---|---|
| Does OpenDesk need to enforce who can start a RustDesk session? | Dashboard RBAC alone does not prevent a direct RustDesk connection if ID/password access still works. | Lookup-only, endpoint-enforced, network-enforced, or future client/server integration. |
| Are endpoint passwords shared outside the dashboard today? | Shared secrets weaken any dashboard-only access model. | Rotate/manage, retire, or accept with documented risk. |
| Must operators get passwordless one-click access? | This would require a secret-management design. | Required, equivalent, or retired. |
| Is session audit required or is launch-intent audit enough? | Logs can show useful events, but OpenDesk launch intent is not proof of a completed session. | Launch-only, log ingestion, or deeper integration. |

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

Cutover remains blocked until each row above is no longer `unknown` and every `required` row maps to validation evidence.
