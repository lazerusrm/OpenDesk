# Requirements

This document defines what OpenDesk must satisfy before it can replace RustDesk Server Pro in production.

The requirement IDs are stable and should be referenced from implementation tasks, tests, and validation evidence.

## Product Requirements

| ID | Requirement | Acceptance Evidence |
|---|---|---|
| PR-001 | OpenDesk must provide a web admin console for device, user, deployment, and health workflows. | Screenshots or manual validation of the console, plus passing UI/API tests for core workflows. |
| PR-002 | OpenDesk must maintain a searchable device inventory keyed by RustDesk ID and internal UUID. | CRUD/search validation passes; exported inventory matches database state. |
| PR-003 | OpenDesk must provide an address-book workflow that replaces daily Pro address-book usage before cutover. | Operator workflow validation confirms devices can be found and launched/copied without Pro. |
| PR-004 | OpenDesk must support sites, tags, owners, notes, and archived devices. | Data model tests and UI validation cover each field and lifecycle state. |
| PR-005 | OpenDesk must generate configured install/download flows for required endpoint OSes without relying on manual executable renaming. | Client delivery validation passes for every required OS. |
| PR-006 | OpenDesk must support endpoint self-registration with scoped, revocable enrollment tokens. | Enrollment validation proves create/update/revoke behavior and audit logging. |
| PR-007 | OpenDesk must expose server health and readiness status for RustDesk ID/relay services. | Health validation covers DNS, TCP ports, UDP where feasible, service state, and key fingerprint. |
| PR-008 | OpenDesk must support backup and restore of its own data/configuration. | Restore test proves a fresh instance can recover expected data. |
| PR-009 | OpenDesk must provide audit logs for admin actions, endpoint enrollment, generated deployment artifacts, and auth events. | Audit validation shows actor, action, object, timestamp, source, and outcome. |
| PR-010 | OpenDesk must provide role-based access control for dashboard/API workflows before production cutover. | Permission matrix tests prove admin/operator/read-only behavior. |
| PR-011 | OpenDesk must clearly distinguish dashboard authorization from RustDesk session authorization. | Docs and UI copy avoid false claims; enforcement tests cover only implemented controls. |
| PR-012 | OpenDesk must preserve official RustDesk clients as the default delivery path. | Client delivery tests verify official signatures/checksums where applicable. |
| PR-013 | OpenDesk must not require RustDesk Server Pro infrastructure at cutover. | Cutover checklist shows no required workflow calls Pro-only services. |
| PR-014 | OpenDesk must support Android and iOS RustDesk app operator workflows. | Mobile validation proves operators can configure the official mobile apps and connect to test endpoints through the expected server. |

## Security Requirements

| ID | Requirement | Acceptance Evidence |
|---|---|---|
| SR-001 | Public repository content must not contain production domains, IPs, tokens, keys, endpoint IDs, customer names, or private topology. | Sensitive-string scan passes outside ignored paths. |
| SR-002 | Runtime secrets must live outside Git in environment/config/secret manager storage. | Config review shows no committed secrets and documented secret locations. |
| SR-003 | Enrollment tokens must be scoped, expiring, revocable, and stored hashed or otherwise protected. | Token lifecycle tests and schema review. |
| SR-004 | API sessions must use secure cookies or bearer tokens with expiration and CSRF protection where applicable. | Auth tests and security review. |
| SR-005 | OpenDesk must not store unattended access passwords in plaintext. | Schema and code review prove no plaintext storage path; managed access uses ADR-008 if required. |
| SR-006 | Generated installer/config scripts must not embed long-lived privileged secrets. | Script inspection tests and audit events. |
| SR-007 | Audit logs must avoid recording full secrets, passwords, private keys, or full enrollment tokens. | Log redaction tests. |
| SR-008 | Production admin UI/API must require HTTPS. | Deployment validation proves HTTP redirects/refuses plaintext. |
| SR-009 | Backups must document whether they contain sensitive material and how they are protected. | Backup metadata and restore documentation. |
| SR-010 | Any claim of RustDesk session access enforcement must be backed by endpoint, network, client, or protocol evidence. | Enforcement evidence exists or feature remains explicitly non-enforcing. |
| SR-011 | OpenDesk should support optional passkey login hardening for admin/operator accounts. | Passkey registration/login tests pass on supported browsers and password fallback/recovery policy is documented. |

## Operational Requirements

| ID | Requirement | Acceptance Evidence |
|---|---|---|
| OR-001 | OpenDesk must deploy via a repeatable Compose-based deployment for the first production target. | `compose.yml`, environment template, and deploy validation. |
| OR-002 | OpenDesk must run independently from RustDesk `hbbs`/`hbbr` so dashboard failure does not break existing remote sessions. | Failure-mode test shows RustDesk sessions still route when OpenDesk is stopped. |
| OR-003 | OpenDesk must expose diagnostics useful for LXC/Proxmox operations. | Health page and runbook cover service state, logs, disk, backup, and port checks. |
| OR-004 | OpenDesk must support a parallel-run period beside the existing RustDesk Pro workflow. | Pilot plan and validation evidence. |
| OR-005 | OpenDesk must have a documented rollback path before cutover. | Rollback drill passes or is explicitly documented as low-risk due to no traffic takeover. |
| OR-006 | OpenDesk must have an upgrade procedure that preserves database and config. | Upgrade validation passes on a test instance. |
| OR-007 | OpenDesk must record upstream RustDesk versions used in client delivery validation. | Release/version records attached to validation matrix. |
| OR-008 | OpenDesk must run CI checks for privacy, docs consistency, generated scripts, and application tests as the project matures. | GitHub Actions workflows pass on pull requests and `main`. |
| OR-009 | OpenDesk implementation must maintain canonical producer-to-consumer contracts and reject internal shim/legacy/compatibility creep. | Engineering standards review and tests prove canonical field names and boundary-only adapters. |
| OR-010 | OpenDesk implementation must keep source files under documented soft size limits or justify exceptions. | Code review and CI/reporting enforce or warn on size thresholds. |
| OR-011 | OpenDesk must complete the documented research roadmap before production cutover. | Every item in `docs/research-roadmap.md` has evidence, decision, and linked follow-up tasks or validation IDs. |

## Integration Requirements

| ID | Requirement | Acceptance Evidence |
|---|---|---|
| IR-001 | OpenDesk must monitor OSS `hbbs` and `hbbr` rather than modifying them for initial replacement. | Health checks and deployment docs use upstream binaries/images. |
| IR-002 | OpenDesk must import or record the RustDesk server public key/fingerprint without committing real key material. | Local ignored context stores real values; public docs use placeholders. |
| IR-003 | OpenDesk must support generated connection helper strings for default-server and explicit-server cases. | Validation covers copy/open behavior. |
| IR-004 | OpenDesk must validate official client config behavior per OS and version. | Client delivery matrix records pass/fail by OS/package/version. |
| IR-005 | OpenDesk may implement RustDesk-shaped compatibility endpoints only after documenting the client behavior they satisfy. | Upstream findings and compatibility tests link to implementation. |
| IR-006 | OpenDesk must validate official RustDesk client behavior per OS/package/version rather than assuming source-level hooks work in released clients. | Research and D-series validation evidence cover required platforms. |

## Cutover Requirements

| ID | Requirement | Acceptance Evidence |
|---|---|---|
| CR-001 | Every Pro feature used in production today must be mapped to implemented, equivalent, or intentionally retired behavior. | Completed parity map signed off by owner/reviewer. |
| CR-002 | Every Core requirement must have passing validation evidence. | Validation matrix complete for required environments. |
| CR-003 | At least one full backup/restore drill must pass. | Restore evidence from fresh instance. |
| CR-004 | At least one pilot group must run successfully through normal operations. | Pilot report with issues closed or accepted. |
| CR-005 | Rollback must be documented and tested where applicable. | Rollback validation. |
| CR-006 | Public repo privacy scan must pass immediately before release/cutover. | Scan output reviewed and recorded. |
| CR-007 | Required CI workflows must be green for the cutover candidate commit. | GitHub Actions status checks pass. |
| CR-008 | All research roadmap items must have accepted evidence and decisions. | Research roadmap is complete and linked from cutover readiness. |
