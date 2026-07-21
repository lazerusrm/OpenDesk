# Owner Decision Worksheet

This worksheet turns the remaining research gaps into explicit owner decisions. Private supporting evidence can live under ignored `local/research/`.

## Evidence Baseline

The authorized read-only discovery establishes observations, not acceptance:

- The inspected RustDesk Pro data is populated across peers, users, groups and cross-group mappings, address books and linked peers, and audit tables. Population proves that these structures exist; it does not prove weekly use or replacement acceptance.
- The inspected runtime includes the official `hbbs` and `hbbr` services and an admin web service in an LXC validation context. This establishes observed components only; it is not a production deployment signoff.
- No private hostnames, addresses, keys, credentials, or topology are recorded in this public worksheet.

Every row still needs an owner decision and the validation IDs listed in the parity map or validation matrix. Automated tests and read-only discovery do not constitute cutover signoff.

## Decision States

- `required`: OpenDesk must replace this before cutover.
- `equivalent`: A different OpenDesk workflow is acceptable.
- `retired`: The workflow is not needed after cutover.
- `unknown`: Cutover blocker.

## Pro Usage Decisions

| Area | Current Evidence | Required Decision | Default Until Decided |
|---|---|---|---|
| Windows custom clients | Read-only inventory shows Windows-only custom-client records; existence is not proof of current use. | Required, equivalent, or retired. | Required: generated Windows install/config flow. |
| Strategies/policies | Strategy rows include configuration options. | Required, equivalent, or retired. | Required: OpenDesk policy model. |
| Personal address books | Address-book records and linked peers are populated. | Required, equivalent, or retired. | Required: OpenDesk address book/device list. |
| Native RustDesk address book | Native app parity is not yet proven necessary. | Required, equivalent, or retired. | Equivalent: OpenDesk web address book, including mobile operator workflow. |
| Managed/passwordless address-book access | Address-book entries contain hashed secret material. | Required, equivalent, or retired. | Unknown; blocks cutover until owner decides. |
| Device/user assignments | Peer, user, and group associations are populated. | Required, equivalent, or retired. | Required: OpenDesk ownership metadata. |
| Control roles | Group and cross-group mapping structures are populated; this does not establish effective operator permissions. | Required, equivalent, or retired. | Equivalent: OpenDesk dashboard/API role model. |
| 2FA | No inspected user rows had 2FA enabled. | Required hardening or retired. | Retired for parity, optional for hardening. |
| Third-party auth | No inspected third-party auth rows were identified. | Required hardening or retired. | Retired for parity, optional for hardening. |
| Passkeys | Desired as soft opt-in OpenDesk auth hardening, especially mobile phone passkeys. | Optional hardening scope and rollout policy. | Optional; not a Pro parity blocker. |
| Audit logs | Connection and console audit tables are populated. | Required, equivalent, or retired. | Required: OpenDesk audit plus optional ingestion. |
| Relay management | Official `hbbr` is present in the observed runtime. | Required, equivalent, or retired. | Required: health/config visibility. |

## Access Model Decision

The owner chose **dashboard/API RBAC only**. OpenDesk roles authorize dashboard and API actions; they do not authorize or deny a RustDesk session. The project must not claim session enforcement without separate endpoint, network, client, or protocol evidence. Validate this boundary with SEC-007 and keep any future enforcement proposal separate.

## Required Validation Platforms

Windows, Linux, and macOS endpoint validation, plus Android and iOS operator workflows, are required before cutover. The observed server/runtime evidence does not substitute for those client and operator tests.

## Access Model Follow-up Questions

| Question | Why It Matters | Required Decision |
|---|---|---|
| Are endpoint passwords shared outside the dashboard today? | Shared secrets weaken a dashboard-only access model. | Rotate/manage, retire, or accept with documented risk. |
| Must operators get passwordless one-click access? | This would require a secret-management design. | Required, equivalent, or retired. |
| Is session audit required or is launch-intent audit enough? | Launch intent is not proof of a completed session. | Launch-only, log ingestion, or deeper integration. |

## Passkey Scope

Passkeys are desired as a soft opt-in feature for OpenDesk login, especially mobile phone passkeys. They should protect access to the OpenDesk dashboard/API only.

Passkeys do not replace RustDesk unattended passwords, endpoint password rotation, native RustDesk session authorization, or any future client/server session-enforcement design.

## Current Recommended Decisions

- Use generated official-client install/config flows instead of Pro custom clients.
- Use the OpenDesk web device list/address book first.
- Do not store plaintext unattended passwords.
- Treat session-level enforcement as out of scope under the dashboard/API-only decision.
- Require Windows/Linux/macOS client validation and Android/iOS operator workflows before cutover.
- Support passkeys as optional OpenDesk login hardening, separate from RustDesk unattended/session passwords.

## Signoff

Cutover remains blocked until each row above is no longer `unknown`, every `required` row maps to validation evidence, and the owner/reviewer records signoff. No signoff is recorded in this worksheet.
