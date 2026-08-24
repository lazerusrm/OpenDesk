# OpenDesk

OpenDesk is a self-hosted control plane around official RustDesk clients and
OSS `hbbs`/`hbbr`. It provides the admin console, inventory, address books,
enrollment, deployment scripts, health probes, backups, and audit log. RustDesk
still owns transport. OpenDesk dashboard roles do not grant or deny sessions.

Use official signed RustDesk apps. This repository does not vendor RustDesk
server or client source. Organization-signed Windows installers are optional
and stay off unless you provision files; see [Install](docs/install.md).

## Install

Follow **[Install OpenDesk](docs/install.md)** for Compose, cargo, production
loopback-plus-TLS posture, and the optional signed Windows package directory.

Quick lab start (plain HTTP, not for the public internet):

```bash
export OPENDESK_CLIENT_TOKEN_HMAC_KEY="$(openssl rand -hex 32)"
read -s OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
docker compose up --build
```

Sign in as `admin` at `http://127.0.0.1:8080/login`, then change the password
on Account. Set ID server, relay, API URL, and hbbs public key under Settings
before generating client scripts.

`OPENDESK_CLIENT_TOKEN_HMAC_KEY` must be a stable random hexadecimal value of
at least 32 bytes. Store it as a deployment secret; rotating it invalidates
every official-client login token.

## Development

Toolchain is Rust 1.88 (`rust-toolchain.toml`). Server-rendered UI (Axum,
Askama, SQLite).

```bash
export OPENDESK_CLIENT_TOKEN_HMAC_KEY="$(openssl rand -hex 32)"
read -s OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_COOKIE_SECURE=false
OPENDESK_LISTEN_ADDR=127.0.0.1:8080 cargo test
OPENDESK_LISTEN_ADDR=127.0.0.1:8080 cargo run
```

Report-only migration review uses `opendesk-migration-dry-run`. It accepts a
sanitized JSON export and never imports or mutates state. See the
[operations runbook](docs/operations-runbook.md).

## Documents

- [Install](docs/install.md)
- [Architecture](docs/architecture.md)
- [Feature checklist](docs/feature-checklist.md)
- [Client delivery](docs/client-delivery.md)
- [Operations runbook](docs/operations-runbook.md)
- [Threat model](docs/threat-model.md)
- [Engineering standards](docs/engineering-standards.md)
- [Cutover readiness](docs/cutover-readiness.md)
- [Pro feature parity](docs/pro-feature-parity.md)
- [Requirements](docs/requirements.md)
- [Validation matrix](docs/validation-matrix.md)
- [Traceability](docs/traceability.md)
- [Software stack](docs/software-stack.md)
- [Architecture decisions](docs/adr.md)
- [CI plan](docs/ci-plan.md)
- [Research roadmap](docs/research-roadmap.md)
- [Owner decisions](docs/research/owner-decisions.md)
