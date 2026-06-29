# Research Status

This table is the current research completion ledger. Public rows summarize decisions; private raw evidence stays under ignored `local/research/`.

Status values:

- `accepted`: enough evidence exists for planning and cutover criteria.
- `partial`: useful evidence exists, but cutover still needs more proof or owner decision.
- `blocked`: cannot close without owner input or a required test environment.

| ID | Status | Evidence | Blocking Gap |
|---|---|---|---|
| R-001 | Accepted | Linux `1.4.8` `.deb` validates `--config` persistence in dev LXC; official docs/source confirm command-line, import, and filename paths; OpenDesk documents generated scripts as the primary path with Linux/Windows renderers merged on `main` (`docs/client-delivery.md`, `docs/dev-validation.md`). macOS/filename/connection-helper artifacts are specified in implementation-handoff and tracked in PR #14 pending merge. Windows/macOS/mobile targets closed via accepted-exception; pilot validation D-001 through D-012, CUT-003. | Resolved: owner-accepted exceptions per closure packets; remaining OS proof at pilot. |
| R-002 | Accepted | Linux `1.4.8` `.deb` validates install, root service, ID readout, restart persistence, and same-version reinstall in dev LXC; OpenDesk `/deployment` publishes Linux/Windows scripts on `main`. macOS script template tracked in PR #14. Windows/macOS closed via accepted-exception with published script templates and pilot validation D-001, D-009, S-009, CUT-003. | Resolved: owner-accepted exceptions; upgrade/silent-install proof deferred to pilot. |
| R-003 | Accepted | Read-only Pro database inspection confirms populated users, groups, device assignments, address books, strategies, Windows custom clients, sessions, console activity, and audit logs; owner decisions in `docs/research/owner-decisions.md` and Production Usage Inventory in `docs/pro-feature-parity.md` have no unresolved Used Today values. | Resolved: owner signoff recorded 2026-06-29 from inspected evidence. |
| R-004 | Accepted | Address-book entries contain hashed secret material; OpenDesk rejects plaintext unattended passwords; ADR-008 defines external secret-manager path; owner decision: equivalent workflow (documented copy/helper path in PR #14 + operator-managed credentials), passwordless one-click retired for cutover. | Resolved: owner decision recorded 2026-06-29. |
| R-005 | Accepted | Current server settings do not require login for RustDesk access; OpenDesk documents lookup-only dashboard RBAC; owner decision: lookup-only, no session-enforcement UI claims. | Resolved: owner decision recorded 2026-06-29. |
| R-006 | Accepted | Linux `1.4.8` `.deb` validates `--deploy --token` request/response contract against controlled dev endpoint (`OK`, `NOT_ENABLED`, `INVALID_INPUT`, `ID_TAKEN`). Owner decision: defer OpenDesk adapter implementation; generated scripts remain primary path; Windows/macOS deploy closed via accepted-exception (unsupported until pilot proves need). | Resolved: implement/defer decision recorded; adapter not required for research closure. |
| R-007 | Accepted | Pro audit tables and `hbbs`/`hbbr` logs provide connection/console/relay visibility; ADR-009 defines audit tiers; owner decision: Tier 1 OpenDesk audit required, Tier 2 ingestion optional Stage 2. | Resolved: owner decision recorded 2026-06-29; ingestion labels validated at implementation. |
| R-008 | Accepted | Official docs support Android manual/QR config; iOS operator-only (not remotely controllable); OpenDesk mobile workflow is generated instructions + operator RustDesk apps. Accepted-exception: pilot validation D-011, D-012, CUT-003 before cutover. | Resolved: owner-accepted exception; pilot proves operator workflows. |
| R-009 | Accepted | Inside-LAN TCP reachability, split public/local DNS, service ports, and log evidence for rendezvous/relay/direct-address behavior confirmed from dev environment. Accepted-exception: WAN/mobile-network/direct-vs-relay proof assigned to pilot S-001 through S-005, CUT-003; config-driven health dashboard specified in implementation-handoff (PR #14 pending merge); plain `/health` on `main` returns `ok` only. | Resolved: owner-accepted exception; health probes do not claim WAN success. |
| R-010 | Accepted | ADR records clean-room control plane default and fork/link/vendor rules. | None unless fork/vendor work is proposed. |

## Closure Rule

An item moves to `accepted` only when its blocking gap is resolved by evidence or explicit owner retirement. If a workflow is retired, record who accepted the retirement and which validation rows no longer apply.

The exact evidence packet required for each research row is defined in [Research Closure Packets](research-closure-packets.md).

## Implementation Handoff

Implementation may proceed while research rows are partial, but production cutover must wait until every row is accepted or explicitly retired. The current implementation handoff is recorded in [Implementation Handoff](implementation-handoff.md).