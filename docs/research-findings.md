# Research Findings

This is the public, sanitized summary of current replacement research. Site-specific evidence and raw outputs live under ignored `local/research/`.

## Completed Evidence Collection

The current production-style deployment was inspected read-only. Evidence confirms:

- RustDesk Server Pro features are present, not just OSS rendezvous/relay.
- Native services run `hbbs` and `hbbr`.
- API/web console behavior is exposed through a reverse proxy.
- The database contains address books, address-book entries, users, groups, roles, control roles, strategies, custom clients, sessions, settings, and audit tables.
- Audit and session-related tables are populated.
- Server logs expose rendezvous, direct-address fetch, relay pairing/closure, startup settings, and wrong-key events.
- Current production-style usage includes users, groups, assigned devices, address-book entries, strategies, custom clients, sessions, and audit history.
- No current users have 2FA enabled in the inspected database, and third-party auth rows were absent.
- Address-book entries include hashed secret material; OpenDesk rejects plaintext unattended passwords and uses equivalent operator workflows.
- Released Linux client evidence covers current AppImage version readout and RustDesk `1.4.8` Debian package install/config/restart/reinstall persistence in a dev LXC.
- A separate dev LXC/VM is the preferred place to validate package installs, deploy compatibility, config persistence, and failure modes.
- Current custom-client usage is Windows-only in the inspected database.
- Strategy rows include active config options, so strategies/policies remain replacement scope unless explicitly retired.
- Role/user-role mappings and third-party auth rows were empty in the inspected database, and no populated OIDC/LDAP/SMTP/API-token style integration tables were identified.
- Console audit rows include recent activity, so web-console workflows remain in scope until owner review retires or replaces them.
- Connection audit rows include end-time and IP/name metadata often enough to support an ingestion design, but not enough to replace first-party OpenDesk audit.
- Local RustDesk source inspection provides mappings for console audit object and operation codes; these should be validated against observed Pro rows before user-facing labels are treated as final.
- Inside-LAN TCP reachability from the dev LXC to the required RustDesk ports is confirmed.
- DNS probing confirms a split public/local resolution shape for the service hostname, and TCP probes from this environment reached the expected RustDesk service ports through local resolution.
- Current server service commands, data paths, public-key fingerprint, database path, settings, and runtime paths are recorded in ignored private evidence.

## Current Decisions

| Research ID | Decision | Status | Remaining Pilot Work |
|---|---|---|---|
| R-001 | Generated scripts primary; filename fallback convenience; Linux `.deb` config validated; Windows/macOS/mobile accepted-exception until pilot. | Accepted | Pilot: D-001 through D-012, CUT-003 per OS. |
| R-002 | Generated install/config scripts primary; Linux package lifecycle validated; Windows/macOS accepted-exception until pilot. | Accepted | Pilot: silent install, upgrade, service/user context D-001, D-009, S-009. |
| R-003 | Populated Pro features mapped in owner decisions and parity inventory; weekly-use assumptions recorded from inspected evidence. | Accepted | Pilot operator confirmation CUT-003. |
| R-004 | Equivalent workflow: connection helpers + operator credentials; no plaintext storage; ADR-008 if managed-password parity later required. | Accepted | None for research closure. |
| R-005 | Lookup-only dashboard access; no RustDesk session-enforcement claims. | Accepted | RBAC matrix at Phase 5 cutover gate. |
| R-006 | Linux deploy contract validated; defer OpenDesk `/api/devices/deploy` adapter; scripts remain primary. | Accepted | Pilot only if adapter scope reopens E-006, E-007. |
| R-007 | Tier 1 OpenDesk audit required; Tier 2 RustDesk ingestion optional Stage 2. | Accepted | Ingestion label validation if Tier 2 selected. |
| R-008 | Mobile RustDesk apps required for operators; Android QR/manual; iOS operator-only. | Accepted | Pilot D-011, D-012, CUT-003. |
| R-009 | LAN TCP + split DNS confirmed; health dashboard config-driven probes; WAN/mobile accepted-exception. | Accepted | Pilot S-001 through S-005, CUT-003 from real clients. |
| R-010 | Clean-room control plane remains the license posture. | Accepted | ADR-007 records fork/link/vendor rules. |

## Sources

- RustDesk client configuration documentation.
- RustDesk client deployment documentation.
- RustDesk Android client documentation.
- RustDesk self-host documentation.
- RustDesk Server Pro web console documentation.
- Local ignored RustDesk OSS source inspection.
- Local read-only production-style server evidence under ignored `local/research/`.
- Dev LXC RustDesk `1.4.8` Debian package validation recorded in `docs/dev-validation.md`.