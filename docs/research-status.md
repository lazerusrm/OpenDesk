# Research Status

This table is the current research completion ledger. Public rows summarize decisions; private raw evidence stays under ignored `local/research/`.

Status values:

- `accepted`: enough evidence exists for planning and cutover criteria.
- `partial`: useful evidence exists, but cutover still needs more proof or owner decision.
- `blocked`: cannot close without owner input or a required test environment.

| ID | Status | Evidence | Blocking Gap |
|---|---|---|---|
| R-001 | Partial | Linux `.deb` package validates command-line config and service config persistence. Official docs/source support other config paths. | Validation lab evidence for Windows installer/portable, macOS, Android, iOS, and Linux GUI/operator workflow. |
| R-002 | Partial | Linux `.deb` package validates install, service creation, ID readout, restart persistence, and same-version reinstall persistence. | Validation lab evidence for Windows/macOS silent install, service/user config, and upgrade behavior. |
| R-003 | Partial | Pro database shows populated users, groups, device assignments, personal address books, strategies, Windows custom clients, sessions, recent console activity, and audit logs. Role mappings and third-party auth integrations were not populated in inspected evidence. | Owner signoff on weekly usage and retired workflows. |
| R-004 | Partial | Address-book entries contain hashed secret material; plaintext storage is rejected; ADR-008 defines external secret-manager path if managed access is required. | Owner decision on passwordless/managed-password requirement. |
| R-005 | Partial | Current settings do not require login for RustDesk access; dashboard RBAC is documented as non-enforcing for sessions. | Owner decision on lookup-only vs real session enforcement. |
| R-006 | Partial | Linux `.deb` validates deploy request shape and response cases against a controlled dev endpoint. | Validation lab evidence for Windows/macOS deploy behavior and future adapter tests. |
| R-007 | Partial | Audit database/log evidence proves useful connection, console, relay, and rendezvous visibility; recent console activity is present; local RustDesk source provides console action mappings; ADR-009 defines audit tiers. | Owner decision on required audit tier and validation of ingestion/labels if selected. |
| R-008 | Partial | Official docs support Android manual/QR config and identify iOS as not remotely controllable. Mobile RustDesk operator apps are required for cutover. | Manual validation of Android and iOS operator workflows with OpenDesk-generated config/instructions. |
| R-009 | Partial | Service ports, logs, inside-LAN TCP reachability, split public/local DNS shape, and local-resolution TCP reachability are validated. | Validation lab evidence for WAN/mobile-network/NAT/direct-vs-relay behavior from real clients. |
| R-010 | Accepted | ADR records clean-room control plane default and fork/link/vendor rules. | None unless fork/vendor work is proposed. |

## Closure Rule

An item moves to `accepted` only when its blocking gap is resolved by evidence or explicit owner retirement. If a workflow is retired, record who accepted the retirement and which validation rows no longer apply.

The exact evidence packet required for each research row is defined in [Research Closure Packets](research-closure-packets.md).

## Implementation Handoff

Implementation may proceed while research rows are partial, but production cutover must wait until every row is accepted or explicitly retired. The current implementation handoff is recorded in [Implementation Handoff](implementation-handoff.md).
