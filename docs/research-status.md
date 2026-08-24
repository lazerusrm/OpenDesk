# Research Status

This table is the current research completion ledger. Public rows summarize authorized read-only discovery and decisions; private raw evidence stays under ignored `local/research/`.

Status values:

- `accepted`: enough evidence exists for planning and cutover criteria.
- `partial`: useful evidence exists, but cutover still needs more proof or owner decision.
- `blocked`: cannot close without owner input or a required test environment.

The authorized discovery confirms populated Pro data structures for peers, users, groups/mappings, address books/linked peers, and audit tables, plus observed official `hbbs`/`hbbr` and admin-web runtime components. It intentionally does not publish counts, private topology, or production signoff.

| ID | Status | Established evidence | Still-required validation or decision |
|---|---|---|---|
| R-001 | partial | Linux package/config evidence and documented official-client configuration paths exist. | Execute released-client validation on Windows, Linux, and macOS endpoints, plus Android/iOS operator workflows; include Linux GUI/operator behavior. |
| R-002 | partial | Linux package install, service creation, ID readout, restart persistence, and same-version reinstall evidence exist. | Validate Windows/macOS silent install, service/user config, upgrades, and required mobile operator setup. |
| R-003 | partial | Read-only discovery confirms populated peer, user, group/mapping, address-book, strategy, custom-client, session, console-activity, and audit structures; role mappings and third-party auth were not established. | Owner must classify weekly use and retire/equate workflows. Map every used capability to a passing validation or recorded retirement; no signoff yet. |
| R-004 | partial | Address-book entries contain hashed secret material; plaintext storage is rejected; ADR-008 defines an external secret-manager path if managed access is required. | Owner must decide whether passwordless/managed-password access is required, equivalent, or retired. |
| R-005 | partial | Owner selected dashboard/API RBAC only. Documentation and implementation tests must preserve that this does not enforce RustDesk sessions. | Validate SEC-007 wording and record endpoint/network/client evidence only if session enforcement is proposed later. Resolve password-sharing and session-audit follow-ups. |
| R-006 | partial | Linux deploy request shape and response cases are validated against a controlled endpoint. | Validate deploy behavior on Windows/macOS and complete isolated compatibility-adapter tests before any compatibility claim. |
| R-007 | partial | Read-only audit tables/logs show connection, console, relay, rendezvous, and recent console visibility; ADR-009 defines audit tiers. | Owner must select launch-only, ingestion, or deeper audit tier; validate labels/ingestion if selected. Do not treat launch intent as session proof. |
| R-008 | partial | Official documentation supports Android manual/QR configuration and identifies iOS as not remotely controllable; mobile apps remain operator scope. | Manually validate Android and iOS operator workflows using OpenDesk-generated instructions/configuration. |
| R-009 | partial | Observed service/runtime evidence and controlled probes establish service reachability and baseline DNS/TCP behavior without publishing topology. | Real Windows/Linux/macOS clients must validate WAN, mobile-network, NAT, direct-vs-relay, and failure behavior. |
| R-010 | accepted | ADR records the clean-room control-plane boundary and fork/link/vendor rules. | None unless fork/vendor work is proposed. |

## Closure Rule

An item moves to `accepted` only when its blocking gap is resolved by evidence or explicit owner retirement. If a workflow is retired, record who accepted the retirement and which validation rows no longer apply.

The exact evidence packet required for each research row is defined in [Research Closure Packets](research-closure-packets.md).

## Implementation Handoff

Implementation may proceed while research rows are partial, but production cutover must wait until every row is accepted or explicitly retired. The current implementation handoff is recorded in [Implementation Handoff](implementation-handoff.md).
