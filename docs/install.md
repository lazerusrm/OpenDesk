# Install OpenDesk

OpenDesk is a self-hosted **control plane** for official RustDesk clients and
OSS `hbbs`/`hbbr`. It is not a remote-desktop engine. Sessions still use
RustDesk transport. Dashboard roles authorize this console and its API only;
they do not grant or deny a RustDesk session.

This guide uses placeholders such as `rd.example.com`. Do not put real
hostnames, addresses, or secrets in the repository.

## What you install

| Piece | Role | Source |
|---|---|---|
| OpenDesk | Inventory, login, address books, enrollment, health | This repository |
| `hbbs` | RustDesk ID / rendezvous | Official RustDesk server OSS (optional fetch) |
| `hbbr` | RustDesk relay | Official RustDesk server OSS (optional fetch) |
| Reverse proxy | TLS for the dashboard and `/api` | Your proxy (production) |
| Official clients | Endpoints and operator apps | RustDesk releases |

OpenDesk does **not** vendor `hbbs`/`hbbr`. The fetch script pulls unmodified
upstream images or zips. Organization-signed Windows packages are optional via
`OPENDESK_SIGNED_CLIENT_DIR`.

## Requirements

- Git, and either **Docker Compose** (recommended) or Rust **1.88**
- A persistent data directory (Compose volume or `OPENDESK_DATA_DIR`)
- For production: a TLS terminator and published `hbbs`/`hbbr` ports

`hbbs`/`hbbr` are **not** a prerequisite. Fetch them in the lab recipe, or
point `/setup` at servers you already run.

On first start with an empty data directory, OpenDesk writes `opendesk.hmac`
next to SQLite when `OPENDESK_CLIENT_TOKEN_HMAC_KEY` is unset. Keep that
directory. Losing the key signs every official client out.

## Lab (recommended)

Same host, plain HTTP, Docker. First image build compiles OpenDesk and can take
several minutes. Do **not** publish port 8080 on a network.

```bash
git clone https://github.com/lazerusrm/OpenDesk.git
cd OpenDesk
bash scripts/fetch-rustdesk-server.sh --yes --start
```

That pulls official `rustdesk/rustdesk-server:latest` and starts OpenDesk plus
`hbbs`/`hbbr`. Compose without `--profile rustdesk-server` starts OpenDesk only.

Copy the hbbs public key (created after hbbs starts once):

```bash
docker compose --profile rustdesk-server exec hbbs cat /root/id_ed25519.pub
```

Open `http://127.0.0.1:8080/setup` (or `/login`, which redirects there when no
users exist). Username is always `admin`. Lab field values:

| Field | Lab value |
|---|---|
| Password | at least 8 characters |
| Public URL | `http://127.0.0.1:8080` (http, not https) |
| ID server | `127.0.0.1` |
| Relay server | `127.0.0.1` |
| API server | `http://127.0.0.1:8080` (same as Public URL) |
| hbbs public key | paste `id_ed25519.pub`, or leave empty and set it later under Settings |

Sign in as `admin`. Open **Deployment** for the official-client import string
and OS scripts.

`127.0.0.1` only works on this machine. Phones or other PCs need this host's
LAN or public name in ID/relay, and firewall access to **21115–21119/tcp** and
**21116/udp**. Set `OPENDESK_RELAY_HOST` to that name before starting hbbs so
clients receive the right relay.

TTY without `--yes` asks whether to fetch `hbbs`/`hbbr` (default no).
`--method github` downloads the official zip, checks sha256, and writes
binaries under `data/rustdesk-server/`. Prefer Docker.

## Lab: cargo (no Docker)

```bash
export OPENDESK_LISTEN_ADDR=127.0.0.1:8080
export OPENDESK_COOKIE_SECURE=false
export OPENDESK_DATA_DIR=data
cargo run
```

`/login` opens the setup wizard. Fetch or install official `hbbs`/`hbbr`
yourself, then use the same lab field values. Later starts reuse
`data/opendesk.hmac`. Reset a forgotten password with
`opendesk-user-password-reset` in the [operations runbook](operations-runbook.md).

## Production shape

1. Run official `hbbs`/`hbbr` (Compose profile `rustdesk-server`, or binaries
   you already run). Point hbbs at the **public** relay
   (`hbbs -r rd.example.com:21117`).
2. Bind OpenDesk to loopback: `OPENDESK_LISTEN_ADDR=127.0.0.1:8080`.
3. Terminate TLS on a reverse proxy. Forward the dashboard and official-client
   `/api` to OpenDesk. Keep 21115–21119 on `hbbs`/`hbbr`, not on OpenDesk.
4. Set `OPENDESK_PUBLIC_BASE_URL` to the HTTPS origin, such as
   `https://rd.example.com`. Leave `OPENDESK_COOKIE_SECURE` true.
5. Keep the data directory (`opendesk.hmac` + SQLite).
6. `/setup`: Public URL and API are the HTTPS origin; ID and relay are the
   public hostname; paste `id_ed25519.pub`.
7. Create operator users and access groups. Unassigned devices stay admin-only.

Do not publish the OpenDesk listen port. Heartbeat and sysinfo posts stay
unauthenticated by design; they are not session proof.

## After login

- **Deployment** — official client links, import string, QR, OS scripts.
- **Account** — enroll the onboard authenticator, then share `/onboard`.
- **Onboard** — technician six-digit code, then install. No recipient account
  is created. Sign in on the official client; enable start-on-boot if the
  machine must stay reachable. Signed Windows `setup.exe` appears only when
  `OPENDESK_SIGNED_CLIENT_DIR` is provisioned.
- **Devices / address books** — visibility grants. Official clients that log in
  see the same scoped list.
- **Status** — DNS/TCP probes, not WAN/UDP proof.
- **Backup** — authenticated JSON export. No in-process scheduler.

## Optional Windows signed installer

Authenticode is an external process. If you already produce a signed wrapper,
set `OPENDESK_SIGNED_CLIENT_DIR` to a directory containing `windows-setup.exe`
and/or `windows-setup.msi`. Otherwise those links stay hidden.

## Out of scope

OIDC, LDAP, SMTP, a password vault, vendored AGPL RustDesk, in-process
Authenticode, and RustDesk session ACL enforcement are not part of this path.
