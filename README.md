# OpenDesk

Initial planning workspace for a self-hosted RustDesk OSS control plane.

Goal: keep the official RustDesk clients and OSS `hbbs`/`hbbr` remote access stack, then build a full RustDesk Server Pro replacement control plane before any production cutover. OpenDesk should provide the management value currently expected from Pro through our own web app, API, deployment tooling, inventory, policy, monitoring, backups, and validation discipline.

Production target:

- RustDesk domain: `rd.example.com`
- Current host context: Proxmox LXC on `root@LAN_HOST`
- Remote access engine: RustDesk OSS server and official RustDesk clients
- Management layer: custom app developed in this repository

## Documents

- [Initial Tape-Out](docs/initial-tapeout.md)
- [Requirements](docs/requirements.md)
- [Architecture](docs/architecture.md)
- [Feature Checklist](docs/feature-checklist.md)
- [Validation Matrix](docs/validation-matrix.md)
- [Client Delivery Plan](docs/client-delivery.md)
- [Upstream Findings](docs/upstream-findings.md)
- [Pro Feature Parity Map](docs/pro-feature-parity.md)
- [Threat Model](docs/threat-model.md)
- [Architecture Decisions](docs/adr.md)
- [Cutover Readiness](docs/cutover-readiness.md)
- [CI Plan](docs/ci-plan.md)
- [Traceability](docs/traceability.md)
- [Engineering Standards](docs/engineering-standards.md)

## Current Decision

Use the official signed RustDesk apps wherever possible. Do not fork the RustDesk client unless an important workflow cannot be solved through external management, install automation, or endpoint self-registration.

Forking or vendoring RustDesk OSS server/client code is deferred until a specific full-replacement requirement cannot be met through the external control plane, deployment automation, endpoint agent, or compatible APIs.

Local upstream reference clones may exist in `upstream/`, which is intentionally ignored by Git.
