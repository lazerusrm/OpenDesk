# OpenDesk

OpenDesk is a self-hosted control plane around official RustDesk clients and
OSS `hbbs`/`hbbr`. It provides the admin console, inventory, address books,
enrollment, deployment scripts, health probes, backups, and audit log. RustDesk
still owns transport. OpenDesk dashboard roles do not grant or deny sessions.

Use official signed RustDesk apps. This repository does not vendor RustDesk
server or client source. Organization-signed Windows installers are optional;
see [Install](docs/install.md).

## Lab install

Docker, same host, plain HTTP. Do not publish this on the internet. The first
build can take several minutes.

```bash
git clone https://github.com/lazerusrm/OpenDesk.git
cd OpenDesk
bash scripts/fetch-rustdesk-server.sh --yes --start
docker compose --profile rustdesk-server exec hbbs cat /root/id_ed25519.pub
```

Open `http://127.0.0.1:8080/setup`. Username is `admin`.

- Public URL and API: `http://127.0.0.1:8080`
- ID server and relay: `127.0.0.1`
- Paste the public key (or leave it empty and add it later under Settings)

Without `--yes`, the fetch script asks whether to pull official `hbbs`/`hbbr`
(AGPL, from RustDesk, not this tree). Default is no. Compose **without**
`--profile rustdesk-server` starts OpenDesk only.

A client-token HMAC key is written next to SQLite when
`OPENDESK_CLIENT_TOKEN_HMAC_KEY` is unset. Rotating it invalidates official-client
login tokens.

Production (loopback + TLS + public relay hostname) is in
**[Install OpenDesk](docs/install.md)**.

## Development

Toolchain is Rust 1.88 (`rust-toolchain.toml`). Server-rendered UI (Axum,
Askama, SQLite).

```bash
export OPENDESK_COOKIE_SECURE=false
OPENDESK_LISTEN_ADDR=127.0.0.1:8080 cargo run
```

`cargo test` is the developer suite, not part of install.

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
