# Install OpenDesk

OpenDesk is a self-hosted **control plane** for official RustDesk clients and
OSS `hbbs`/`hbbr`. It is not a remote-desktop engine. Sessions still use
RustDesk transport. Dashboard roles authorize this console and its API only;
they do not grant or deny a RustDesk session.

This guide is the public self-host path. It uses placeholders such as
`rd.example.com`. Do not put real hostnames, addresses, or secrets in the
repository.

## What you install

| Piece | Role | Source |
|---|---|---|
| `hbbs` | RustDesk ID / rendezvous server | Official RustDesk server OSS |
| `hbbr` | RustDesk relay | Official RustDesk server OSS |
| OpenDesk | Inventory, login, address books, enrollment, health | This repository |
| Reverse proxy | TLS for the dashboard and `/api` | Your proxy (Caddy, nginx, or equivalent) |
| Official clients | Endpoints and operator apps | RustDesk releases |

OpenDesk does **not** vendor `hbbs`/`hbbr` or ship a signed Windows installer by
default. Organization-signed Windows packages are optional: set
`OPENDESK_SIGNED_CLIENT_DIR` to a directory that contains `windows-setup.exe`
and/or `windows-setup.msi`. When that directory is unset, the UI offers official
downloads plus generated PowerShell, Linux, and macOS scripts.

## Requirements

- Linux host (or Docker) with a persistent data directory
- Rust **1.88** if you build from source (`rust-toolchain.toml`)
- Official `hbbs` and `hbbr` binaries and a generated ID keypair
- TLS terminator for any network-facing deployment
- `openssl` (or equivalent) to generate a client-token HMAC key

Generate a stable HMAC key once and keep it with other deployment secrets.
OpenDesk stores only HMAC-SHA-256 digests of official-client access tokens.
Changing this key signs every official client out.

```bash
openssl rand -hex 32
```

## Lab: Compose

From a clone of this repository, export a bootstrap admin password and the HMAC
key, then build:

```bash
export OPENDESK_CLIENT_TOKEN_HMAC_KEY="$(openssl rand -hex 32)"
read -s OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
docker compose up --build
```

Compose publishes OpenDesk on host port 8080 for local trials. It sets
`OPENDESK_COOKIE_SECURE` false so a browser can keep the session on plain HTTP.
The bootstrap username is `admin`. Sign in at `http://127.0.0.1:8080/login`,
then change the password on **Account**.

This published port is **lab-only**. Do not expose it on a network. Production
must bind OpenDesk to loopback and terminate HTTPS on a reverse proxy.

## Lab: cargo

```bash
export OPENDESK_CLIENT_TOKEN_HMAC_KEY="$(openssl rand -hex 32)"
read -s OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_BOOTSTRAP_ADMIN_PASSWORD
export OPENDESK_BOOTSTRAP_ADMIN_USERNAME=admin
export OPENDESK_LISTEN_ADDR=127.0.0.1:8080
export OPENDESK_PUBLIC_BASE_URL=http://127.0.0.1:8080
export OPENDESK_COOKIE_SECURE=false
export OPENDESK_DATA_DIR=data
cargo test
cargo run
```

The first process start creates `admin` when the database has no users. Later
starts ignore the bootstrap password. Reset a forgotten password with
`opendesk-user-password-reset` as documented in the
[operations runbook](operations-runbook.md).

## Production shape

1. Install official `hbbs` and `hbbr`. Generate `id_ed25519` / `id_ed25519.pub`
   with `rustdesk-utils`. Point `hbbs` at your relay (`hbbs -r rd.example.com:21117`
   or the equivalent documented by the server package).
2. Put OpenDesk on loopback, for example `OPENDESK_LISTEN_ADDR=127.0.0.1:8080`.
3. Terminate TLS on a reverse proxy. Forward the dashboard and official-client
   `/api` to OpenDesk. Keep `hbbs`/`hbbr` ports on the ID and relay listeners,
   not on the OpenDesk process.
4. Set `OPENDESK_PUBLIC_BASE_URL` to the public HTTPS origin, such as
   `https://rd.example.com`.
5. Leave `OPENDESK_COOKIE_SECURE` at its default true (or set it true explicitly).
6. Store `OPENDESK_CLIENT_TOKEN_HMAC_KEY` and the bootstrap password outside the
   repository (systemd `EnvironmentFile`, compose env file, or equivalent).
7. Sign in as `admin`, open **Account**, and set a password you will keep.
8. Open **Settings** and save ID server, relay server, API server, and the hbbs
   public key. Generated scripts and import strings use those values.
9. Create operator users and access groups as needed. Unassigned devices stay
   admin-only.

Do not publish the OpenDesk listen port. Heartbeat and sysinfo compatibility
posts stay unauthenticated by design; do not treat them as session proof.

## After login

- **Deployment** — official client links, import string, QR, and OS scripts.
- **Onboard** (`/onboard`) — technician six-digit authenticator code, then
  install. The Windows one-click file appears only when a signed package is
  provisioned. Otherwise use the PowerShell script. Installing does not create
  a recipient account. Sign in on the official client and enable start-on-boot
  if the machine must stay reachable.
- **Devices / address books** — inventory and visibility grants. Official
  clients that log in see the same scoped device list.
- **Status** — DNS/TCP probes and public-key fingerprint, not WAN/UDP proof.
- **Backup** — authenticated JSON export. There is no in-process scheduler.

## Optional Windows signed installer

Authenticode signing is an external process. OpenDesk never signs binaries.

If you already produce a signed wrapper, set `OPENDESK_SIGNED_CLIENT_DIR` to an
existing directory and place allowlisted files `windows-setup.exe` and/or
`windows-setup.msi` there. The deployment and onboard pages then offer those
downloads. If the variable is unset, those links stay hidden.

## Out of scope for this install

OIDC, LDAP, SMTP, a password vault, vendored AGPL RustDesk, in-process
Authenticode, and RustDesk session ACL enforcement are not part of this path.
