# Architecture Decision Record

This file records current architectural decisions. Update it when a major decision changes.

## ADR-001: Build a Control Plane, Not a Remote Desktop Engine

Decision: OpenDesk will use official RustDesk clients and OSS `hbbs`/`hbbr` for remote desktop transport.

Rationale:

- Remote desktop transport is high-risk and platform-specific.
- RustDesk already handles screen capture, input injection, relay, NAT traversal, file transfer, and mobile clients.
- The missing value is management/control plane behavior.

Consequences:

- OpenDesk can move faster.
- Some Pro-like behavior inside the native RustDesk client may remain unavailable without compatibility work or a fork.

## ADR-002: Public Repository With Ignored Private Context

Decision: The repo is public. Private deployment context lives under ignored `local/` files.

Rationale:

- The owner is comfortable with public code.
- Public docs must not leak site-specific details.
- Local contributors and automation still need private context for accurate work.

Consequences:

- Every commit needs a privacy scan.
- Public docs use placeholders.

## ADR-003: Official Client Delivery First

Decision: OpenDesk will first generate install/config flows for official RustDesk clients.

Rationale:

- Official clients preserve upstream signing, updates, and mobile distribution.
- Custom clients create signing, packaging, and maintenance burden.

Consequences:

- Native client UX is constrained.
- If official client config hooks are insufficient, wrapper/fork options remain available.

## ADR-004: OpenDesk-Native API First

Decision: Build OpenDesk-native APIs and only implement RustDesk-shaped compatibility endpoints when validated and necessary.

Rationale:

- Pro API compatibility may be unstable or license/terms-sensitive.
- OpenDesk can satisfy most management workflows externally.

Consequences:

- The initial address book is web-based, not native-client-based.
- Compatibility endpoints must be isolated and explicitly tested.

## ADR-005: No Plaintext Unattended Password Storage

Decision: OpenDesk will not store unattended RustDesk passwords in plaintext.

Rationale:

- Password storage materially increases risk.
- External secret managers are better suited for this class of secret.

Consequences:

- Password-managed address book parity requires a later secret-management design.
- Replacement acceptance must either avoid this workflow or implement it defensibly.

## ADR-006: Cutover Requires Full Replacement Evidence

Decision: OpenDesk can be developed in stages, but production cutover is blocked until the full replacement gate passes.

Rationale:

- The goal is to replace Pro, not merely prove a small MVP.
- Remote access tooling has high operational risk.

Consequences:

- Validation evidence matters as much as implementation.
- Feature parity map and requirements must stay current.

## ADR-007: Keep RustDesk Code At The Boundary

Decision: OpenDesk will remain a clean-room control plane unless a future requirement explicitly justifies fork/link/vendor work.

Rationale:

- RustDesk client/server upstream code is AGPL-licensed.
- OpenDesk can satisfy most replacement workflows through its own database, API, generated configuration, endpoint registration, monitoring, and official client commands.
- Keeping RustDesk code at the boundary avoids coupling OpenDesk internals to upstream implementation details.

Allowed by default:

- Run upstream RustDesk binaries/images unchanged.
- Read official documentation.
- Inspect ignored local upstream clones as reference.
- Generate configuration/scripts for official clients.
- Monitor `hbbs`/`hbbr` services and logs.

Requires an ADR update before implementation:

- Forking RustDesk client or server.
- Copying RustDesk source into OpenDesk.
- Linking OpenDesk directly against RustDesk AGPL libraries.
- Vendoring RustDesk code.
- Redistributing modified RustDesk binaries.

Consequences:

- Native-client parity features may take longer if they require a fork.
- License obligations stay explicit instead of accidentally creeping into core OpenDesk code.

## ADR-008: Externalize Managed Access Secrets

Decision: If passwordless or managed-password RustDesk access is required, OpenDesk will store references to an external secret manager, not the unattended passwords themselves.

Rationale:

- Current evidence shows address-book entries contain hashed secret material.
- OpenDesk is a public control-plane project and should not become the primary vault for unattended remote-access secrets.
- Secret rotation, recovery, access audit, and encryption-at-rest are specialized responsibilities.

Allowed by default:

- Store vault item references, labels, and non-secret metadata.
- Record audit events for secret reference creation, update, use intent, and removal.
- Generate scripts that rotate endpoint passwords when backed by a reviewed secret-management flow.

Not allowed by default:

- Store plaintext unattended RustDesk passwords in OpenDesk tables.
- Log full passwords, full vault tokens, or full recovery material.
- Embed long-lived privileged vault tokens in generated scripts.

Acceptable secret managers:

- A dedicated vault service with API access controls.
- A password manager with auditable item access.
- A deployment-specific secret store approved before implementation.

Consequences:

- Native passwordless parity remains blocked until the owner decides it is required.
- If required, implementation needs a separate secret-manager integration design and validation rows before cutover.

## ADR-009: Audit Tiers Must Be Explicit

Decision: OpenDesk audit will be split into explicit tiers so UI and reports do not overclaim RustDesk session proof.

Rationale:

- OpenDesk can prove actions it performs itself.
- RustDesk database/log evidence can show useful session and relay events.
- A dashboard launch event is not proof that a remote session started or ended.

Audit tiers:

- Tier 1: OpenDesk first-party audit for login, device edits, enrollment, generated artifacts, settings, and owner decisions.
- Tier 2: RustDesk server database/log ingestion for connection, relay, rendezvous, and console events where available.
- Tier 3: Client-side or deeper client/server integration if stronger session proof is required.

Consequences:

- Tier 1 is required for OpenDesk implementation.
- Tier 2 is optional but recommended for Pro audit parity.
- Tier 3 requires a later design and must not be implied by Tier 1 or Tier 2 evidence.

## ADR-010: Rust Control Plane By Default

Decision: OpenDesk will use Rust as the default backend/API/control-plane implementation language. The detailed stack is locked in `docs/software-stack.md`.

Rationale:

- OpenDesk is security-sensitive remote-access management software.
- Rust gives strong type boundaries for generated scripts, endpoint enrollment, auth, audit, and config rendering.
- A single service binary is operationally attractive for LXC/Compose deployment.
- Rust aligns with the surrounding RustDesk ecosystem while ADR-007 keeps RustDesk source code at the boundary.

Default implementation shape:

- Rust HTTP service.
- SQLite first with a clear Postgres migration path.
- Server-rendered UI first.
- TypeScript only for UI workflows that genuinely need richer client behavior.
- Compatibility endpoints isolated from the OpenDesk-native domain model.

Consequences:

- Build and CI should use Rust tooling by default.
- Any non-Rust backend needs a new ADR.
- Using Rust does not permit copying, linking, or vendoring RustDesk AGPL code without an ADR update.
