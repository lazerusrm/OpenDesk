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

Current finding:

Read-only database inspection confirms populated production-style usage of users, groups, assigned devices, address books, strategies, custom clients, sessions, and audit logs. This makes those features replacement-scope unless the owner explicitly retires the workflow.

Deeper inventory:

- Custom-client rows are Windows-only in the inspected database.
- Strategy rows include config options and are not empty placeholders.
- Address books are mostly personal, with entries linked to known devices.
- Control-role rows exist, but inspected mappings were empty.
- Role, user-role, and role-scope rows were empty in the inspected database.
- Third-party auth rows were empty, and no inspected table names indicated populated OIDC, LDAP, SMTP, API-token, or token-integration state.
- Console audit rows show recent web console activity within the last 180 days, so web-console workflows are not purely historical.

Not currently proven:

- Which populated features are used weekly.
- Whether custom clients are still needed after generated install/config flows exist.
- Whether native RustDesk address books are required if the OpenDesk web address book is complete.
- Whether strategies are historical configuration or active policy.

Decision:

Treat every populated Pro feature as required until owner review marks it replaced, equivalent, or retired.

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

Current finding:

Address-book entry metadata includes hashed secret material. It does not expose plaintext passwords in the inspected evidence, but it is enough to classify passwordless/managed-password parity as security-sensitive.

Decision:

OpenDesk must not store plaintext unattended passwords. ADR-008 requires externalizing managed access secrets if native passwordless or managed-password workflow is required for cutover.

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

Current finding:

Current server settings do not require login for access, while device records are assigned to users. This means web-console ownership/visibility and actual RustDesk connection authorization are separate concerns.

Decision:

OpenDesk may claim dashboard access control only for OpenDesk UI/API workflows. It must not claim RustDesk session-level enforcement unless endpoint settings, password rotation, network controls, or future client/server integration actually enforce it.

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

Current finding:

The inspected database has connection, console, file, and alarm audit history. Server logs expose startup settings, direct-address lookup, punch-hole attempts, relay pairing/closure, and wrong-key events.

Audit detail:

Most connection audit rows include end times. Connection audit metadata includes IP/name fields, while console audit metadata includes changed object identifiers and names. This is enough to design useful ingestion, but OpenDesk still needs first-party audit for actions it performs itself.

Console audit detail:

Console audit activity is grouped by numeric object/action types in the Pro database. The inspected metadata keys identify changed object names, name lists, and object IDs. Local RustDesk source inspection found helper mappings for console object types and operation codes, covering group, user, device, and address-book management operations. OpenDesk can use those mappings as a starting point for ingestion labels, but ingestion tests should still verify them against observed Pro console rows before cutover.

Decision:

OpenDesk should implement first-party audit for admin actions, deployment artifacts, enrollment, auth, and settings changes. ADR-009 defines separate audit tiers for OpenDesk actions, RustDesk log/database ingestion, and any future deeper session proof.

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

Current finding:

The server exposes the expected RustDesk service ports for ID, relay, API, and websocket traffic. Logs show direct-address lookup, TCP/UDP punch-hole behavior, relay pairing, and relay closure events.

Network detail:

Inside-LAN TCP reachability from the dev LXC to the expected RustDesk service ports is confirmed. Additional DNS probing shows public DNS resolves the service host to a public address while local resolution returns a private LAN address, matching a split-DNS or hairpin-avoidance topology. TCP probes from this environment reached the expected RustDesk service ports through local resolution; HTTPS on the default TLS port was not reachable in that probe. This still does not prove real WAN, mobile-network, or direct-vs-relay behavior from official clients.

Decision:

OpenDesk health checks should cover DNS, TCP listeners, UDP where feasible, public key fingerprint, service state, log freshness, and disk/database health. Real LAN/WAN/NAT/split-DNS behavior still needs client-side validation before cutover.

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
