# Threat Model

OpenDesk manages remote access metadata and deployment flows. A compromise can make remote access easier for an attacker even if OpenDesk does not directly carry RustDesk session traffic.

## Assets

- OpenDesk admin accounts and sessions.
- Device inventory and RustDesk IDs.
- Site/customer/device metadata.
- Enrollment tokens.
- Generated installer/config scripts.
- Audit logs.
- Backup archives.
- RustDesk server public key/fingerprint.
- RustDesk server private key, if ever present locally. It should not be.
- Optional secret-manager references.

## Trust Boundaries

| Boundary | Risk |
|---|---|
| Browser to OpenDesk | Session theft, CSRF, auth bypass, exposed metadata. |
| OpenDesk to endpoint scripts | Token leakage, script tampering, wrong server config. |
| OpenDesk to RustDesk OSS server | Health checks may reveal topology; key material must not leak. |
| Public repo to local deployment | Private values accidentally committed. |
| Backup storage | Inventory and token material exposure. |
| Operator to RustDesk client | OpenDesk launch intent does not guarantee RustDesk session authorization. |

## Primary Threats

| ID | Threat | Mitigation |
|---|---|---|
| T-001 | Production secrets committed to public Git. | `.gitignore`, `CODEX.md`, local context folder, sensitive-string scan before commits. |
| T-002 | Enrollment token used after intended deployment. | Expiring scoped tokens, revocation, hashed storage, audit events. |
| T-003 | Generated script modified in transit. | HTTPS, checksums for cached installers, visible script preview, short-lived tokens. |
| T-004 | OpenDesk user sees unauthorized device metadata. | RBAC and site/tag scoping tests. |
| T-005 | OpenDesk claims session ACLs that it does not enforce. | Explicit access boundary docs and UI wording; no enforcement claims without tests. |
| T-006 | Backup leaks full device inventory or tokens. | Backup classification, encryption option, token hashing, restore docs. |
| T-007 | RustDesk private key copied into OpenDesk repo or logs. | Never ingest private key; only record public fingerprint; scan logs/docs. |
| T-008 | Endpoint check-in spoofed. | Enrollment tokens, optional device key/pairing later, duplicate detection, audit anomalies. |
| T-009 | Official RustDesk installer supply-chain issue. | Version pinning, signature/checksum validation, cached known-good packages where appropriate. |
| T-010 | Admin account compromise. | Strong passwords, optional reverse-proxy SSO/2FA, audit logs, session expiration. |

## Required Security Decisions

- Enrollment tokens are write-only secrets. Store only hashes or irreversible digests.
- Generated scripts should show enough content for audit, but should not expose full token values after initial creation.
- Public docs must use `example.com` and placeholder hostnames.
- Production site context belongs only under ignored local paths or runtime configuration.
- OpenDesk should not store unattended passwords unless there is a dedicated design using external secret management.

## Security Validation

Security validation must cover:

- Auth required for all admin/device APIs.
- CSRF protection or same-site cookie strategy.
- Token scope and revocation.
- Redaction in logs and audit events.
- Backup contents and restore behavior.
- Public repo sensitive scan.
- Generated script secret scan.
- Permission matrix by role/site/tag.

