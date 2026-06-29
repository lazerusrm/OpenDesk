# Implementation Handoff

This document translates the research roadmap into implementation guardrails. It is intentionally public and sanitized; site-specific values and raw validation evidence stay under ignored `local/`.

## Locked Direction

OpenDesk is a clean-room Rust control plane around official RustDesk clients and OSS `hbbs`/`hbbr` services.

Primary implementation choices:

- Rust service using the stack in [Software Stack](software-stack.md).
- Server-rendered operational UI first.
- SQLite first, with schema/query discipline that keeps a Postgres path open.
- Official RustDesk clients remain the supported desktop/mobile clients.
- Generated deployment artifacts replace executable-renaming and Pro custom-client dependence where validation proves the flow.
- RustDesk-shaped compatibility endpoints are isolated from OpenDesk-native APIs.
- OpenDesk RBAC controls OpenDesk workflows only unless a later enforcement design proves RustDesk session control.

## First Implementation Slices

Implement in this order unless new evidence changes the risk profile:

1. Core app boot, health page, database migration, and first admin login.
2. Device inventory, server config, and first-party audit events.
3. Enrollment tokens and generated Linux/Windows deployment artifacts.
4. RustDesk-shaped deploy endpoint adapter under an isolated route group.
5. Address-book/operator workflow that launches or copies official-client connection details.
6. Mobile operator config instructions and Android QR payload generation.
7. Optional passkey login hardening for OpenDesk accounts.
8. RustDesk database/log ingestion if Tier 2 audit parity remains required.

## Non-Negotiable Boundaries

- Do not store plaintext unattended passwords.
- Do not claim RustDesk session enforcement from dashboard visibility, launch-intent audit, or address-book filtering.
- Do not copy, link, or vendor RustDesk AGPL source without a new ADR.
- Do not mix OpenDesk-native domain contracts with RustDesk compatibility request/response shapes.
- Do not introduce internal fallback fields or compatibility aliases for unshipped OpenDesk contracts.
- Do not commit private deployment values, production evidence, generated secrets, build output, or local workflow notes.

## Research Gates

Research rows R-001 through R-010 are `accepted` in `docs/research-status.md` as of 2026-06-29. Production cutover still requires pilot validation for accepted-exception OS/network targets.

| Research ID | Status | Implementation Impact | Remaining Cutover Proof |
|---|---|---|---|
| R-001 | Accepted | Generated scripts are the primary config path; Linux/Windows renderers merged on `main`; macOS/filename/helpers in PR #14. | Pilot validation D-001 through D-012, CUT-003 for Windows/macOS/mobile. |
| R-002 | Accepted | Linux/Windows scripts on `/deployment` (`main`); macOS template in PR #14. | Pilot silent install, upgrade, and service/user context D-001, D-009, S-009. |
| R-003 | Accepted | Populated Pro features mapped in owner decisions and parity inventory. | Pilot operator confirmation CUT-003. |
| R-004 | Accepted | Equivalent operator workflow; no plaintext unattended passwords; ADR-008 if managed-password parity later required. | None for research closure. |
| R-005 | Accepted | Lookup-only dashboard access; no session-enforcement UI claims. | RBAC matrix SEC-008 at Phase 5 cutover gate. |
| R-006 | Accepted | Linux deploy contract validated; defer OpenDesk adapter; scripts remain primary. | Reopen adapter only if pilot proves need E-006, E-007. |
| R-007 | Accepted | Tier 1 OpenDesk audit required; Tier 2 RustDesk ingestion optional Stage 2. | Ingestion label validation if Tier 2 selected. |
| R-008 | Accepted | Mobile RustDesk apps required for operators; generated instructions/QR path. | Pilot D-011, D-012, CUT-003. |
| R-009 | Accepted | Config-driven health dashboard in PR #14; `main` has plain `/health` only; probes must not claim WAN success. | Pilot S-001 through S-005, CUT-003 from real clients. |
| R-010 | Accepted | Clean-room license posture. | New ADR only if fork/link/vendor work is proposed. |

## CI Gates

Before implementation is merged, CI should prove:

- `cargo fmt --check` passes.
- `cargo test` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes once the first scaffold is stable.
- Database migrations apply to a fresh SQLite database.
- Generated deployment artifacts render deterministically from test-only inputs.
- Documentation checks and privacy scans pass.
- Build output and local evidence paths are ignored.

Before cutover, CI or staging validation should additionally prove:

- Container or LXC deployment starts cleanly.
- Health checks cover OpenDesk, database, RustDesk services, configured ports, disk, backups, and log freshness.
- Backup and restore are tested.
- Generated artifacts contain no production secrets.
- Validation matrix rows tied to required workflows have accepted evidence.

## Handoff Notes

Implementation should prefer boring, direct domain names from [Engineering Standards](engineering-standards.md). If a RustDesk API uses a different term, map it at the boundary and keep the OpenDesk contract canonical.

Any new implementation work should update at least one of:

- [Requirements](requirements.md)
- [Validation Matrix](validation-matrix.md)
- [Traceability](traceability.md)
- [Research Status](research-status.md)
- [CI Plan](ci-plan.md)

Use ignored private notes for raw host output, credentials, production topology details, packet captures, logs, and screenshots.
