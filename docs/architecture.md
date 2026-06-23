# Architecture

OpenDesk is a control plane around RustDesk OSS. It must avoid owning the remote desktop engine unless a future requirement proves that client/server integration is unavoidable.

## Context

```text
Operators
  browser + official RustDesk client
        |
        | HTTPS
        v
OpenDesk
  web UI
  API
  auth/session management
  device inventory
  deployment generation
  health/backup/audit
        |
        | generated scripts / enrollment API
        v
Managed endpoints
  official RustDesk client
  optional OpenDesk registration script/service
        |
        | RustDesk rendezvous/relay/session traffic
        v
RustDesk OSS
  hbbs
  hbbr
```

## Components

## Implementation Stack

OpenDesk should be implemented as a Rust service by default.

The locked stack is recorded in [Software Stack](software-stack.md).

Default choices:

- Rust backend/API/control-plane service.
- Server-rendered HTML for the initial operational UI.
- SQLite for early development and small deployments.
- Postgres migration path if concurrency or reporting needs justify it.
- TypeScript frontend only when a workflow cannot stay clean as server-rendered UI.

RustDesk code remains at the boundary. Using Rust for OpenDesk does not imply linking to, vendoring, or copying RustDesk AGPL source.

### Web UI

Responsibilities:

- Device/address-book search.
- Device detail pages.
- Site/tag/user administration.
- Download/configuration flows.
- Health dashboard.
- Audit log review.
- Backup/restore operations.

The UI should be utilitarian and dense enough for repeated operational use. It should not be a marketing site.

### API

Responsibilities:

- Authenticated admin/operator/read-only workflows.
- Endpoint enrollment and check-in.
- Generated deployment artifacts.
- Health status.
- Backup/export.
- Audit event ingestion.
- Optional compatibility endpoints where validated.

API shape should be OpenDesk-native first. Compatibility endpoints should be isolated under an explicit namespace or handler group so they can be tested and removed independently.

### Database

Early development can use SQLite. Production can remain SQLite if concurrency and backup behavior are sufficient, but Postgres should be available as a migration path.

Required tables:

- `users`
- `roles`
- `sessions`
- `devices`
- `sites`
- `tags`
- `device_tags`
- `address_books`
- `address_book_entries`
- `enrollment_tokens`
- `endpoint_checkins`
- `deployment_artifacts`
- `server_configs`
- `health_checks`
- `audit_events`
- `backup_records`

### Endpoint Registration

The first endpoint registration mechanism should be script-based:

1. Install or locate official RustDesk client.
2. Apply OpenDesk-managed server settings.
3. Read RustDesk ID where supported.
4. Collect safe metadata: hostname, OS, architecture, RustDesk version.
5. POST to OpenDesk using a scoped enrollment token.
6. Repeat periodically or on demand for check-in.

The registration script/service must not send unattended passwords or private keys.

### Client Delivery

OpenDesk should support these delivery flows:

- Generated install/config script.
- Cached official installer plus generated config script.
- Filename-based custom server fallback where validated.
- Wrapper installer only if the script path is not good enough.
- Client fork only if a required replacement workflow cannot be solved otherwise.

### RustDesk OSS Services

OpenDesk monitors but does not initially replace:

- `hbbs`
- `hbbr`
- RustDesk client session transport

Failure of OpenDesk should not break existing RustDesk rendezvous/relay behavior unless an optional future feature intentionally couples them.

## Data Flow: New Endpoint

```text
Admin opens OpenDesk download page
  -> chooses OS/site/tags/enrollment scope
  -> downloads or copies generated command
Endpoint runs command
  -> installs official RustDesk
  -> applies server config
  -> reads RustDesk ID/version
  -> POST /api/enrollments/check-in
OpenDesk
  -> validates token
  -> creates/updates device
  -> writes audit event
  -> shows device in address book
Operator
  -> finds device
  -> copies/opens RustDesk connection
```

## Data Flow: Operator Connection

```text
Operator logs into OpenDesk
  -> searches address book
  -> opens device detail
  -> copies ID or launches helper
Official RustDesk client
  -> uses configured OSS server
  -> connects to endpoint
OpenDesk
  -> may log launch intent
  -> does not claim session enforcement unless separately integrated
```

## Access Control Boundary

OpenDesk RBAC controls:

- Who can see devices in OpenDesk.
- Who can edit metadata.
- Who can generate enrollment artifacts.
- Who can view audit logs/backups.
- Who can administer users/settings.

OpenDesk RBAC does not automatically control:

- Whether RustDesk accepts a session to an endpoint.
- Whether a known RustDesk ID/password can be used outside OpenDesk.
- Whether a direct peer-to-peer connection is possible.

Session-level enforcement requires one or more of:

- Endpoint password rotation and vaulting.
- Endpoint-side policy enforcement.
- Network restrictions.
- Client fork/patch.
- Protocol/server integration.

## Deployment Topology

Recommended initial production topology:

```text
LXC or VM
  reverse proxy
  opendesk app
  opendesk database
  backups

Existing or separate LXC/VM
  hbbs
  hbbr
```

OpenDesk may live alongside RustDesk OSS services, but the preferred operational model is independent services with clear backups and failure boundaries.

## Compatibility Strategy

Order of preference:

1. Official documented/supported client configuration behavior.
2. Existing official client behavior validated from source and real packages.
3. OpenDesk endpoint registration script/service behavior.
4. Isolated compatibility endpoint for RustDesk-shaped calls.
5. Light client fork.
6. Deep client/server fork.

The project should document why it moved down this list before doing so.
