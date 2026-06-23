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
- Local agents still need private context for accurate work.

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

