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

## Current Decisions

| Research ID | Decision | Remaining Work |
|---|---|---|
| R-001 | Official docs and source support manual config, import/export, command-line config, automatic config, and filename config. Command-line config uses the same parser as filename config and sets ID server, key, API server, and relay server values. | Run Windows/Linux/macOS/mobile released-client validation matrix. |
| R-002 | Generated scripts using official client commands are the primary deployment path. Source confirms install, service install, config import, config string, ID readout, ID set, password set, assignment, and deploy commands exist. | Confirm silent flags, config persistence, service/user config, and upgrade behavior on real released clients. |
| R-003 | Pro features are actively present and must be treated as replacement scope. | Fill owner-facing usage inventory with which features are used weekly. |
| R-004 | Address-book/password behavior is in scope for design. | Decide whether passwordless/managed-password workflow is required. |
| R-005 | Dashboard RBAC alone cannot be treated as session enforcement. | Decide whether session-level enforcement is required and how to implement it. |
| R-006 | Deploy endpoint compatibility is source-backed and worth prototyping behind an isolated adapter. It still cannot be treated as complete without stock released-client testing. | Test `--deploy --token` against a controlled endpoint and record exact request/response behavior. |
| R-007 | OpenDesk can provide native audit and likely parse server/pro audit sources, but full session semantics need design. | Define audit tiers and ingestion strategy. |
| R-008 | Mobile workflow remains untested. | Decide whether mobile is required for cutover. |
| R-009 | Standard RustDesk ports and direct/relay behavior are confirmed. | Add LAN/WAN/NAT/split-DNS validation evidence. |
| R-010 | Clean-room control plane remains the license posture. | ADR-007 records fork/link/vendor rules. |

## Sources

- RustDesk client configuration documentation.
- RustDesk client deployment documentation.
- RustDesk self-host documentation.
- RustDesk Server Pro web console documentation.
- Local ignored RustDesk OSS source inspection.
- Local read-only production-style server evidence under ignored `local/research/`.
