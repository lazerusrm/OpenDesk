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
| R-001 | Official docs and source support several config paths, but released-client behavior must be tested per OS/package/version. | Run Windows/Linux/macOS/mobile client validation matrix. |
| R-002 | Generated scripts using official client commands are the primary deployment path. | Confirm silent flags, config persistence, service/user config, and upgrade behavior on real clients. |
| R-003 | Pro features are actively present and must be treated as replacement scope. | Fill owner-facing usage inventory with which features are used weekly. |
| R-004 | Address-book/password behavior is in scope for design. | Decide whether passwordless/managed-password workflow is required. |
| R-005 | Dashboard RBAC alone cannot be treated as session enforcement. | Decide whether session-level enforcement is required and how to implement it. |
| R-006 | Deploy endpoint compatibility is promising but not closed. | Test `--deploy --token` against a controlled endpoint. |
| R-007 | OpenDesk can provide native audit and likely parse server/pro audit sources, but full session semantics need design. | Define audit tiers and ingestion strategy. |
| R-008 | Mobile workflow remains untested. | Decide whether mobile is required for cutover. |
| R-009 | Standard RustDesk ports and direct/relay behavior are confirmed. | Add LAN/WAN/NAT/split-DNS validation evidence. |
| R-010 | Clean-room control plane remains the license posture. | ADR-007 records fork/link/vendor rules. |

## Sources

- RustDesk client configuration documentation.
- RustDesk client deployment documentation.
- RustDesk self-host documentation.
- RustDesk Server Pro web console documentation.
- Local read-only production-style server evidence under ignored `local/research/`.
