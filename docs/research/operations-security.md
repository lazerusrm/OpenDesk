# Operations And Security Research

## R-003 Current RustDesk Pro Usage Inventory

Question:

Which RustDesk Server Pro features are actually used in production today?

Must inventory:

- Address books.
- Managed passwords/passwordless address-book connections.
- Device groups/security groups.
- Access control rules.
- Strategies/policies.
- Connection logs and other logs.
- Users/groups/admin roles/control roles.
- Two-factor authentication.
- OIDC/LDAP/SMTP.
- Relay management.
- Custom client builds.
- API integrations.
- Web console workflows used weekly.

Output:

- Completed Production Usage Inventory in `docs/pro-feature-parity.md` or private evidence under `local/`.
- Each used feature mapped to OpenDesk replacement, equivalent workflow, or explicit retirement decision.

Related validation:

- CUT-003.
- CR-001.
- RS-003.

## R-004 Address Book And Password Model

Question:

Does production rely on RustDesk address books for password-managed or passwordless connections?

Must determine:

- Whether unattended passwords are stored in RustDesk Pro today.
- Whether operators expect one-click/passwordless connection from the address book.
- How RustDesk Pro address books relate to device groups and security groups in current usage.
- Whether external secret manager integration is required for replacement.
- Whether password rotation or per-device credential policy is required.

Output:

- Secret-management design decision.
- If password management is required, an ADR and threat-model update before implementation.

Related validation:

- R-005.
- SEC-005.
- SR-005.
- RS-004.

## R-005 Access Control Reality

Question:

What enforcement is required beyond OpenDesk dashboard RBAC?

Must determine:

- Whether production depends on Pro device-group ACLs.
- Whether endpoint passwords are shared broadly today.
- Whether knowing a RustDesk ID/password outside OpenDesk must be prevented.
- Whether enforcement can be achieved through endpoint password rotation, network controls, registration service, or client/server integration.
- What access claims the UI may safely make.

Output:

- Access enforcement decision.
- RBAC matrix.
- If session-level enforcement is required, an implementation design before cutover.

Related validation:

- SEC-007.
- SEC-008.
- R-005.
- RS-005.

## R-007 Session And Audit Log Sources

Question:

What session/audit visibility can OpenDesk provide without modifying RustDesk clients or OSS server?

Must determine:

- What `hbbs` logs expose.
- What `hbbr` logs expose.
- Whether logs identify session start/end, relay use, peer IDs, or IPs.
- What official client logs expose per OS.
- Whether client-side log collection is acceptable.
- Whether OpenDesk can provide launch-intent audit only, or real session audit.

Output:

- Audit capability matrix.
- Decision on launch-intent-only vs log ingestion vs future client integration.

Related validation:

- C-007.
- PR-009.
- RS-007.

## R-009 Relay Scaling, NAT, LAN, And DNS Behavior

Question:

What network behavior must OpenDesk document and validate for the deployment?

Must determine:

- Required public ports for the chosen RustDesk topology.
- Router/firewall/NAT forwarding.
- Hairpin NAT behavior from inside the LAN.
- Split-DNS requirement.
- Direct vs relay behavior on LAN, WAN, and remote networks.
- Whether additional relay servers are required.
- Whether relay selection/geographic routing matters.
- Whether WebSocket ports are required.

Output:

- Network topology/runbook.
- Health-check requirements.
- Decision on extra relay support.

Related validation:

- S-001 through S-005.
- R-003.
- RS-009.

## R-010 Legal And License Posture

Question:

What license boundary keeps OpenDesk clean-room, and what changes if we fork or link RustDesk code?

Must determine:

- Which upstream RustDesk components are AGPL.
- Whether OpenDesk will use upstream binaries/images unchanged.
- Whether OpenDesk will copy, link, vendor, or modify upstream code.
- Source publication obligations if modified network services are deployed.
- Notice/license requirements for any redistributed artifacts.

Output:

- License posture ADR.
- Clear rule for when legal review is required.

Related validation:

- CI-009 public content hygiene.
- ADR updates before fork/vendor work.
- RS-010.

