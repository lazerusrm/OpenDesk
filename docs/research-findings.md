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
- Address-book entries include hashed secret material, so passwordless/managed-password parity must be treated as a security-sensitive requirement until owner signoff retires it.
- Released Linux client evidence now covers current AppImage version readout and current Debian package install/config/restart/reinstall persistence in a dev LXC.
- A separate dev LXC/VM is the preferred place to validate package installs, deploy compatibility, config persistence, and failure modes.

## Current Decisions

| Research ID | Decision | Remaining Work |
|---|---|---|
| R-001 | Official docs/source support multiple config paths. Current released Linux `.deb` validates command-line config writing ID server, key, API server, and relay server values into root service config. | Run Windows/macOS/mobile released-client validation matrix and Linux GUI/operator workflow tests. |
| R-002 | Generated scripts using official client commands are the primary deployment path. Current released Linux `.deb` validates package install, root service creation, ID readout, service restart persistence, and same-version reinstall persistence. | Confirm Windows/macOS silent flags, config persistence, service/user config, and upgrade behavior on real released clients. |
| R-003 | Pro features are actively present and several are populated: users, groups, assigned devices, address books, strategies, custom clients, sessions, and audit logs. | Owner must confirm which populated features are actually used weekly and which may be retired. |
| R-004 | Address-book entries include hashed secret material. OpenDesk must not store plaintext unattended passwords, and any managed-password parity needs a dedicated secret-management design. | Owner must decide whether passwordless/managed-password workflow is required for cutover. |
| R-005 | Current settings do not require login for access, so dashboard RBAC alone cannot be treated as RustDesk session enforcement. | Decide whether session-level enforcement is required and how to implement it. |
| R-006 | Deploy endpoint compatibility is source-backed and worth prototyping behind an isolated adapter. It still cannot be treated as complete without stock released-client testing. | Test `--deploy --token` against a controlled endpoint and record exact request/response behavior. |
| R-007 | Database audit tables and server logs provide useful visibility; OpenDesk can provide native admin/enrollment audit plus optional RustDesk log/database ingestion. | Define exact audit tiers and avoid claiming full session enforcement from launch-intent events alone. |
| R-008 | Mobile workflow remains untested. | Decide whether mobile is required for cutover. |
| R-009 | Standard RustDesk ports, service state, direct-address lookup, punch-hole behavior, and relay pairing events are confirmed. | Add LAN/WAN/NAT/split-DNS validation evidence from real clients. |
| R-010 | Clean-room control plane remains the license posture. | ADR-007 records fork/link/vendor rules. |

## Sources

- RustDesk client configuration documentation.
- RustDesk client deployment documentation.
- RustDesk self-host documentation.
- RustDesk Server Pro web console documentation.
- Local ignored RustDesk OSS source inspection.
- Local read-only production-style server evidence under ignored `local/research/`.
