# Research Closure Packets

This document defines the exact evidence packets needed to move the remaining `partial` research rows to `accepted`. Raw artifacts belong under ignored `local/research/`; public docs should only summarize sanitized conclusions.

## Packet Rules

Every closure packet must include:

- Research ID and validation IDs.
- Date, tester, reviewer, and environment.
- Tested RustDesk client version and package type where applicable.
- Tested OpenDesk commit SHA where applicable.
- Artifact path under ignored `local/research/`.
- Result: `pass`, `fail`, `accepted-exception`, or `retired`.
- If retired or accepted as an exception: owner, reason, and review date.

No packet may include production passwords, private keys, license values, full tokens, private host details, private IPs, screenshots with secrets, or raw production database dumps in public docs.

## R-001 Client Configuration

Purpose: prove which official client config mechanisms work per required OS/package/version.

Required packets:

| Target | Required Evidence | Validation IDs |
|---|---|---|
| Windows installer | Version, installer source/signature, config command or import method, service/user config result, restart persistence. | D-001, D-002, D-006, D-009 |
| Windows portable | Version, filename config result, browser duplicate filename result, elevation limits, persistence result. | D-007, D-008 |
| Linux desktop/operator | Version, GUI/operator workflow, config path, connection to test endpoint. | D-003, D-004, D-009, R-002 |
| macOS | Version, installer source, permission requirements, user/root config result, restart persistence. | D-009, R-002 |
| Android | App version, manual or QR config result, connection to test endpoint. | D-011 |
| iOS | App version, manual config result, operator connection to test endpoint. | D-012 |

Acceptance rule:

R-001 can move to `accepted` when each target has pass evidence, accepted exception, or owner retirement.

## R-002 Deployment Mechanics

Purpose: prove install, configure, verify, restart, reinstall, and upgrade behavior without Pro custom clients.

Required packets:

- Windows installer silent install/config/restart/reinstall or upgrade packet.
- Linux package install/config/restart/reinstall or upgrade packet.
- macOS install/config/restart/reinstall or upgrade packet.
- Per-OS RustDesk ID and version readout result.
- Per-OS config storage location and service/user context result.

Acceptance rule:

R-002 can move to `accepted` when Windows, Linux, and macOS have enough evidence to publish a supported deployment procedure or a documented accepted exception.

## R-003 Current Pro Usage

Purpose: decide which populated Pro workflows must be replaced and which can be retired.

Required packets:

- Owner signoff for each row in [Owner Decision Worksheet](research/owner-decisions.md).
- Production usage summary showing used, equivalent, retired, or unknown for address books, managed passwords, groups, access rules, strategies, logs, users, 2FA, third-party auth, relay management, custom clients, API integrations, and weekly console workflows.
- Validation mapping for every `required` or `equivalent` row.

Acceptance rule:

R-003 can move to `accepted` when no Pro usage area remains `unknown` and every required/equivalent workflow maps to validation evidence or a planned cutover gate.

## R-004 Address Book And Password Model

Purpose: decide whether OpenDesk needs managed/passwordless access and how secrets are handled.

Required packets:

- Owner decision: passwordless/managed-password workflow is `required`, `equivalent`, or `retired`.
- If required: secret-management ADR, threat-model update, backup sensitivity decision, and validation IDs for storage, rotation, and audit.
- If equivalent or retired: signed owner decision explaining the replacement workflow or retirement.

Acceptance rule:

R-004 can move to `accepted` when the password model no longer depends on unresolved assumptions and plaintext unattended password storage remains rejected.

## R-005 Access Enforcement

Purpose: decide whether OpenDesk is lookup/config only or must enforce remote-session authorization.

Required packets:

- Owner decision: lookup-only, endpoint-enforced, network-enforced, or future client/server integration.
- UI/API wording review proving no false session-enforcement claims.
- If enforcement is required: design and validation rows for the chosen enforcement path.

Acceptance rule:

R-005 can move to `accepted` when OpenDesk's access claims exactly match the implemented enforcement mechanism.

## R-006 Deploy Endpoint Compatibility

Purpose: decide whether stock official clients can use an OpenDesk RustDesk-shaped deploy endpoint.

Required packets:

- Linux deploy endpoint evidence already exists; include the artifact path in the closure note.
- Windows client deploy request/response evidence.
- macOS client deploy request/response evidence if the client supports it, or accepted unsupported evidence if not.
- Adapter tests proving accepted response cases, rejected auth, duplicate handling, and no leakage into OpenDesk-native domain contracts.

Acceptance rule:

R-006 can move to `accepted` when required desktop clients either pass deploy compatibility or have documented unsupported/retired decisions, and adapter behavior is tested.

## R-007 Session And Audit Sources

Purpose: decide what audit tier OpenDesk must ship.

Required packets:

- Owner decision: Tier 1 only, Tier 1 plus Tier 2 ingestion, or deeper Tier 3 integration.
- Audit capability matrix for OpenDesk audit, RustDesk database, `hbbs` logs, `hbbr` logs, and any client logs used.
- If Tier 2 is required: ingestion validation showing parsed fields, retention, label mapping, and failure behavior.
- If Tier 3 is required: design and validation plan before cutover.

Acceptance rule:

R-007 can move to `accepted` when audit claims, UI labels, and retention behavior are proven for the selected tier.

## R-008 Mobile Workflow

Purpose: prove official mobile RustDesk apps work with OpenDesk-supported operator workflows.

Required packets:

- Android app version, config method, QR/manual payload result, and connection to test endpoint.
- iOS app version, manual config result, and operator connection to test endpoint.
- Owner decision that mobile is operator-only unless managed mobile endpoints become required.

Acceptance rule:

R-008 can move to `accepted` when Android and iOS operator workflows pass or have accepted exceptions.

## R-009 Network Behavior

Purpose: prove DNS, NAT, LAN, WAN, mobile-network, and direct-vs-relay behavior for real clients.

Required packets:

- LAN official-client connection result, including direct or relay outcome.
- WAN official-client connection result, including direct or relay outcome.
- Mobile-hotspot official-client connection result, including direct or relay outcome.
- DNS evidence from internal and external resolvers.
- TCP and UDP port evidence where feasible.
- NAT/hairpin or split-DNS decision.
- Relay scaling decision: current single relay accepted, additional relay required, or future-only.

Acceptance rule:

R-009 can move to `accepted` when real official clients validate the network paths needed for cutover, not only port probes.

## R-010 License Posture

R-010 is already accepted. Reopen it only if implementation proposes copying, linking, vendoring, modifying, or redistributing RustDesk source/binaries beyond the current ADR boundaries.

## Cutover Closure

Cutover is blocked until:

- Every research row is `accepted`, or a workflow is explicitly retired with owner signoff.
- Every `required` workflow has validation evidence.
- Every `accepted-exception` has owner, reason, and review date.
- Private evidence remains ignored.
- Public docs remain sanitized.
