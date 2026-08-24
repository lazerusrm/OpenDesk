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
docker compose up --build
```

Optional official `hbbs`/`hbbr` (AGPL, pulled from RustDesk, not this tree):

```bash
bash scripts/fetch-rustdesk-server.sh
```

That asks on a TTY. Pass `--yes` to pull `rustdesk/rustdesk-server:latest` and
use `docker compose --profile rustdesk-server up --build`.

Open `http://127.0.0.1:8080/setup`. Username is `admin`. Set the password and
the ID, relay, API, and hbbs public key values. A client-token HMAC key is
written next to SQLite when the env value is unset.

`OPENDESK_CLIENT_TOKEN_HMAC_KEY` remains supported if you want to inject the
key yourself. Rotating it invalidates every official-client login token.

## Development

Toolchain is Rust 1.88 (`rust-toolchain.toml`). Server-rendered UI (Axum,
Askama, SQLite).

```bash
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
