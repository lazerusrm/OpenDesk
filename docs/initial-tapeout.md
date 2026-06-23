# Initial Tape-Out

## Objective

Build a clean-room management/control plane around RustDesk OSS so we can fully replace RustDesk Server Pro for fleet administration before cutover, while continuing to use official RustDesk clients and the existing self-hosted relay/ID infrastructure wherever possible.

The intended product is a full operational replacement for Pro, not merely a thin MVP. It does not need to be a drop-in Pro API clone unless that becomes the best way to satisfy a required workflow. The product must cover the full replacement checklist before production cutover.

## Non-Goals

- Reimplement remote desktop transport.
- Reimplement screen capture, input injection, codecs, file transfer, or NAT traversal.
- Ship custom mobile apps.
- Maintain custom-signed desktop clients as the default path.
- Reverse-engineer RustDesk Pro as the primary integration strategy.
- Store unattended access passwords in plaintext.
- Claim server-enforced RustDesk ACLs unless we actually enforce them at the endpoint, network, or protocol layer.

## Proposed Stack

Recommended initial stack:

- Backend: Rust
- HTTP: Axum on Tokio
- Templates: Askama server-rendered HTML first
- Frontend enhancement: htmx or TypeScript only if the UI grows beyond CRUD/dashboard workflows
- Database: SQLite for early local development, Postgres once multi-user/concurrency/backup requirements justify it
- Auth: local admin user for early development; later Authelia, Authentik, or OIDC behind reverse proxy
- Deployment: Docker Compose
- Reverse proxy: Caddy or Traefik, depending on what is already standard on the server
- Endpoint bootstrap: PowerShell for Windows, shell script for Linux, shell/profile script for macOS

Why Rust:

- Strong fit for a security-sensitive control plane.
- Produces a single deployable service binary.
- Good ecosystem for HTTP services, SQL, templates, crypto, and typed data boundaries.
- Matches the broader RustDesk ecosystem without linking to or vendoring RustDesk AGPL code.
- Keeps canonical producer-to-consumer types practical.

Default implementation shape:

- Rust HTTP service.
- SQLite first, with Postgres-compatible query discipline where practical.
- Server-rendered templates for the operational UI.
- Small isolated compatibility handlers for RustDesk-shaped endpoints.
- Generated PowerShell/shell artifacts rendered from typed Rust config structs.

The detailed stack is locked in `docs/software-stack.md`.

## System Shape

```text
Official RustDesk clients
  Windows / macOS / Linux / Android / iOS
        |
        | RustDesk ID, relay, and session traffic
        v
RustDesk OSS server
  hbbs + hbbr at rd.example.com

Custom OpenDesk
  web UI + API + database
  inventory, address book, config generation, endpoint registration
        |
        | optional endpoint check-in
        v
Managed endpoints
```

## Core Data Model

Initial entities:

- User
- Device
- Site
- Tag
- AddressBook
- AddressBookEntry
- EndpointCheckIn
- DeploymentPackage
- AuditEvent
- ServerHealthCheck

Device fields:

- Internal UUID
- RustDesk ID
- Alias/display name
- Hostname
- OS family/version
- Architecture
- RustDesk client version
- Site/location
- Owner/customer
- Tags
- Notes
- Last check-in timestamp
- Last known LAN IP, optional
- Last known WAN IP, optional
- Created/updated timestamps
- Archived flag

Sensitive fields:

- Do not store unattended passwords in OpenDesk unless a later design uses a defensible secret-management model.
- If password storage is later required, use an external secret manager first: Vaultwarden/Bitwarden, SOPS, age-encrypted export, or HashiCorp Vault.

## Integration Modes

### Mode 1: External Address Book

The dashboard stores device IDs and metadata. Operators copy/click a device ID and connect with the official RustDesk client.

This is the first build path.

### Mode 2: Endpoint Self-Registration

Endpoint script reads local hostname/OS/RustDesk ID and posts it to the control plane with an enrollment token.

This makes the address book populate itself without needing Pro.

### Mode 3: Preconfigured Client Delivery

The frontend provides downloads or scripts that install the official RustDesk client and apply server settings for `rd.example.com`.

This replaces the painful executable-renaming workflow where feasible.

### Mode 4: Light Fork, Deferred

Only if needed, maintain a small RustDesk client fork to bake in defaults, branding, or a dashboard link. Avoid core protocol changes.

## Deployment Layout

Preferred initial layout:

```text
/opt/opendesk/
  compose.yml
  data/
    opendesk.sqlite
    backups/
  config/
    app.env
```

Network:

- `rd.example.com`: RustDesk ID/relay service
- `rd-admin.example.com`: control plane web UI/API
- Keep control plane behind HTTPS and auth.

## Security Model

Initial controls:

- HTTPS only for admin UI/API.
- Admin login required.
- Enrollment tokens for endpoint registration.
- Token scopes: enroll-only, rotateable, revocable.
- Audit every dashboard login, device create/update/archive, generated deployment package, and endpoint enrollment.
- Do not log secrets.
- Exportable backup of database and configuration.

Important limitation:

Dashboard access control does not equal RustDesk session enforcement. If an endpoint accepts a RustDesk connection and the operator knows the ID/password, OSS RustDesk may allow it. Real enforcement must be done through endpoint settings, password hygiene, network boundaries, or future client/server changes.

## Replacement Readiness Definition

OpenDesk is ready for production cutover only when:

- Admin can log in.
- Admin can create/edit/archive devices.
- Admin can search/filter devices.
- Admin can copy RustDesk ID and launch/copy connection details.
- Admin can generate Windows and Linux install/config scripts for `rd.example.com`.
- A test endpoint can self-register.
- Server health page reports RustDesk port reachability.
- Database can be backed up and restored.
- Validation matrix passes on at least one Windows and one Linux endpoint.
- All RustDesk Pro features used in production today are either replicated, intentionally replaced by an equivalent OpenDesk workflow, or explicitly accepted as no longer needed.
- No production cutover occurs until the replacement matrix passes for the required endpoint OSes and operator workflows.

## Open Questions

- Which OSes are mandatory on day one: Windows only, Windows/Linux, or macOS too?
- Do we need mobile operator workflows, or just mobile clients pointed at the relay?
- What existing reverse proxy/auth stack is already on the Proxmox host?
- Are unattended passwords currently stored in RustDesk Pro address books?
- Is one admin enough initially, or do we need multi-user roles immediately?
- Should the first deployment target be the existing LXC or a new LXC?
