# OpenDesk

Initial planning workspace for a self-hosted RustDesk OSS control plane.

Currently V.1 ALPHA -- working with caveats.

Goal: keep the official RustDesk clients and OSS `hbbs`/`hbbr` remote access stack, then build a full RustDesk Server Pro replacement control plane. OpenDesk provides the management value currently expected from Pro through our own web app, API, deployment tooling, inventory, policy, monitoring, backups, and validation discipline.

Production target:

- RustDesk domain: `rd.example.com`
- Current host context: Proxmox LXC on `root@LAN_HOST`
- Remote access engine: RustDesk OSS server and official RustDesk clients
- Management layer: custom app developed in this repository

## Documents

- [Initial Tape-Out](docs/initial-tapeout.md)
- [Requirements](docs/requirements.md)
- [Architecture](docs/architecture.md)
- [Software Stack](docs/software-stack.md)
- [Feature Checklist](docs/feature-checklist.md)
- [Validation Matrix](docs/validation-matrix.md)
- [Validation Lab](docs/validation-lab.md)
- [Client Delivery Plan](docs/client-delivery.md)
- [Upstream Findings](docs/upstream-findings.md)
- [Pro Feature Parity Map](docs/pro-feature-parity.md)
- [Threat Model](docs/threat-model.md)
- [Architecture Decisions](docs/adr.md)
- [Cutover Readiness](docs/cutover-readiness.md)
- [Operations Runbook](docs/operations-runbook.md)
- [CI Plan](docs/ci-plan.md)
- [Traceability](docs/traceability.md)
- [Engineering Standards](docs/engineering-standards.md)
- [Research Roadmap](docs/research-roadmap.md)
- [Research Findings](docs/research-findings.md)
- [Research Status](docs/research-status.md)
- [Owner Decision Worksheet](docs/research/owner-decisions.md)
- [Client Validation Procedures](docs/research/client-validation-procedures.md)
- [Dev Validation Environment](docs/dev-validation.md)

## Current Decision

Use the official signed RustDesk apps wherever possible. Do not fork the RustDesk client unless an important workflow cannot be solved through external management, install automation, or endpoint self-registration.

Forking or vendoring RustDesk OSS server/client code is deferred until a specific full-replacement requirement cannot be met through the external control plane, deployment automation, endpoint registration service, or compatible APIs.

Local upstream reference clones may exist in `upstream/`, which is intentionally ignored by Git.

## Development

The first build slice is a Rust control plane service (Axum, Askama, sqlx/SQLite).

For the separate report-only migration review, build and run
`opendesk-migration-dry-run`; it accepts only a sanitized JSON export, an existing
SQLite file, and explicit `GROUP_ID:SITE_UUID` mappings. It uses a read-only
snapshot and never imports or mutates state. See the [Operations Runbook](docs/operations-runbook.md).

```bash
cargo test
OPENDESK_LISTEN_ADDR=127.0.0.1:8080 cargo run
```

Default bootstrap admin credentials come from `OPENDESK_BOOTSTRAP_ADMIN_USERNAME` and `OPENDESK_BOOTSTRAP_ADMIN_PASSWORD` (change before any real deployment).

Compose deployment:

```bash
docker compose up --build
```
