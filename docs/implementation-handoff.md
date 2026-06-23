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

The implementation can proceed before all research is accepted, but production cutover cannot. The current blockers are:

| Research ID | Implementation Impact | Evidence Needed Before Cutover |
|---|---|---|
| R-001 | Deployment artifacts must remain configurable per OS/package. | Windows installer, Windows portable, macOS, Android, iOS, and Linux GUI/operator validation. |
| R-002 | Generated install/config scripts must be treated as provisional by OS. | Silent install, service/user config location, restart, upgrade, uninstall, and reinstall evidence. |
| R-003 | Feature scope must assume populated Pro features are required. | Owner review of weekly use and explicit retirement of unused workflows. |
| R-004 | Secret-management design must stay out of the core DB unless required. | Owner decision on passwordless or managed-password parity. |
| R-005 | UI copy and authorization checks must avoid false enforcement claims. | Owner decision on lookup-only versus real session enforcement. |
| R-006 | Deploy adapter must be isolated and tested against stock clients. | Windows/macOS deploy behavior and adapter contract tests. |
| R-007 | Audit UI must distinguish OpenDesk audit from RustDesk session evidence. | Owner decision on required audit tier and ingestion validation if selected. |
| R-008 | Mobile support must cover operator workflows. | Android and iOS operator validation with generated config/instructions. |
| R-009 | Health checks must not imply WAN success from LAN-only probes. | WAN, NAT loopback, split-DNS, direct, and relay validation from real clients. |
| R-010 | License posture is accepted. | New ADR only if fork/link/vendor work is proposed. |

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
