# Validation Matrix

## Evidence State

This matrix distinguishes evidence already established from validation still required. `Established` means a current-repository automated test or the authorized read-only discovery baseline supports the statement; it is not a cutover signoff. `Required` means an executed evidence packet is still needed. No row is accepted unless the evidence rules below are satisfied.

Established baseline: current-repository tests cover core dashboard/API workflows, enrollment, audit, backup round-trip, health probes, generated deployment flows, and dashboard/API RBAC. Authorized read-only discovery confirms populated Pro data structures and observed official `hbbs`/`hbbr` plus an admin-web runtime. It does not prove weekly usage, client parity, or production readiness.

Required client scope: Windows, Linux, and macOS endpoint validation, plus Android and iOS operator workflows. Mobile is operator/configuration scope, not a claim of managed mobile endpoints. RBAC is dashboard/API-only and does not enforce RustDesk sessions.

## Environments

Target test environments:

- Windows 10 or 11 endpoint
- Linux desktop endpoint
- macOS endpoint
- Linux server/LXC environment for control plane
- Android operator client, configuration-only
- iOS operator client, configuration-only

## Evidence Rules

No validation row counts as passing until evidence is recorded with:

- Status: `not-run`, `pass`, `fail`, or `accepted-exception`.
- Commit SHA and validation date.
- Environment, OS/version, RustDesk client version, and RustDesk server version where applicable.
- Test operator or reviewer.
- Artifact path or link: screenshot, log excerpt, test output, backup/restore transcript, or signed owner decision.
- Exception owner and expiry/review date for any `accepted-exception`.

## Evidence Classification

The row criteria below remain the acceptance contract. This classification prevents current evidence from being mistaken for cutover acceptance.

| Class | Validation IDs | Meaning in this baseline |
|---|---|---|
| Established implementation evidence | C-001 through C-010, E-001 through E-008, MIG-001 through MIG-004, SEC-002 through SEC-005, SEC-007 through SEC-008, S-005, S-006, S-008 | Current-repository tests or controlled probes cover the stated behavior. Record environment/date/operator evidence before cutover. |
| Established discovery, validation still required | S-001 through S-004, S-009, D-003 through D-004, R-001, R-006, RS-001 through RS-009 | Authorized read-only discovery or partial Linux/config evidence supports scope only. Execute the required endpoint, network, version, or owner decision tests. |
| Required client/operator validation | D-001 through D-016, R-002 through R-006, RS-001 through RS-009 | Windows/Linux/macOS endpoint tests and Android/iOS operator workflows remain required; generated artifacts and source inspection do not substitute for released-client evidence. |
| Required cutover evidence | CUT-001 through CUT-006, RS-003 through RS-009 | Pilot, parallel-run, owner inventory/decisions, network scenarios, and cutover drills remain open. |
| Established policy boundary | SEC-007, RS-005 | Owner selected dashboard/API RBAC only. These rows must ensure no UI or documentation implies RustDesk session enforcement. |
| Accepted research posture | RS-010 | Clean-room control-plane boundary is accepted by ADR; this is not a product or deployment signoff. |
| Established migration safety evidence | MIG-001 through MIG-004 | Strict dry-run parser/reconciliation tests reject sensitive or ambiguous input. An authorized export, explicit mapping evidence, and any separately approved write path remain required. |

No class above marks a cutover row as `pass` by itself. Each executed packet must still include status, commit/date, environment and versions, operator/reviewer, artifact path, and exception details where applicable.


Research rows count as complete only when `docs/research-roadmap.md` contains the decision, `docs/research-status.md` marks the item `accepted`, and the local/public evidence path exists.

## Server Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| S-001 | DNS resolution | Resolve `rd.example.com` externally and internally | Resolves to expected public/private address for test context |
| S-002 | ID server TCP | Connect to required `hbbs` TCP ports | Expected ports accept connections |
| S-003 | Relay TCP | Connect to required `hbbr` TCP ports | Expected relay ports accept connections |
| S-004 | UDP NAT traversal | Validate UDP `21116` path where feasible | UDP path available or documented exception exists |
| S-005 | Public key | Compare dashboard key fingerprint to server key | Fingerprint matches known server public key |
| S-006 | Backup | Create backup archive | Backup contains database/config without runtime junk |
| S-007 | Restore | Restore backup into fresh instance | App starts and data matches source |
| S-008 | Compose deployment | Deploy clean instance from documented Compose config | App starts with persistent data/config and no production secrets |
| S-009 | Upgrade | Apply documented upgrade over populated test instance | Database/config survive and health checks pass |
| S-010 | Server config evidence | Read back redacted ID/relay/API settings and public-key fingerprint | Values match approved test configuration without exposing secrets. |
| S-011 | WAN transport | Connect an enrolled test endpoint and operator across a WAN path | Connection outcome and client/server versions are recorded without private topology. |
| S-012 | NAT/relay fallback | Exercise a NAT case and relay fallback separately | Direct/relay outcome is recorded; unsupported cases are an explicit exception. |

## Control Plane Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| C-001 | Admin auth | Log in/out with admin account | Auth required and session expires as configured |
| C-002 | Device create | Add device with RustDesk ID and alias | Device visible in list/detail |
| C-003 | Device edit | Change alias, site, tags, notes | Changes persist after reload |
| C-004 | Device archive | Archive device | Device hidden from default list and recoverable |
| C-005 | Search | Search by alias/hostname/ID/tag | Expected matching devices returned |
| C-006 | Export CSV | Export device list | CSV opens and includes expected fields |
| C-007 | Audit | Modify a device | Audit event records actor/action/object/time |
| C-008 | Health page | Open health dashboard | Shows DNS, ports, service status, and timestamp |
| C-009 | User administration | Create, disable, and role-assign test users | Login and permissions change according to the selected role |
| C-010 | Deployment page | Generate deployment artifact from UI | Artifact matches selected OS/site/tags/scope and records an audit event |

## Client Delivery Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| D-001 | Windows download page | Generate Windows install command/script | Script includes correct server config and no plaintext secrets |
| D-002 | Windows install | Run script on test Windows endpoint | RustDesk installed/configured without executable renaming |
| D-003 | Linux download page | Generate Linux install command/script | Script includes correct server config and no plaintext secrets |
| D-004 | Linux install | Run script on test Linux endpoint | RustDesk installed/configured without manual server entry |
| D-005 | Version pin | Generate script for pinned client version | Script installs requested tested version |
| D-006 | Signature preservation | Validate official installer signature/checksum | Signature/checksum matches expected source |
| D-007 | Filename config fallback | Download/rename official Windows exe using `host=`, `key=`, `relay=` pattern | Client applies expected server settings or fallback is marked unsupported |
| D-008 | Duplicate filename handling | Test filename config with browser-added `(1)` suffix | Behavior is documented and does not corrupt server/key settings |
| D-009 | Config persistence | Restart RustDesk after scripted config | Server settings persist and connection still uses expected server |
| D-010 | Official release update | Update official client after OpenDesk install | Config remains valid or update limitations are documented |
| D-011 | Android operator app | Configure official Android app with OpenDesk-generated instructions/QR | App uses expected server config and connects to a test endpoint |
| D-012 | iOS operator app | Configure official iOS app with OpenDesk-generated instructions | App uses expected server config and connects to a test endpoint |
| D-013 | macOS install/config | Install current official client on each required macOS architecture and read back config | User/service context behavior and required permissions are recorded. |
| D-014 | Client update persistence | Update each required desktop client and verify ID/config/service state | Version/checksum and before/after readback are recorded. |
| D-015 | Client rollback | Restore the last-known-good official client/config procedure after an update test | Endpoint returns to documented working state without secrets in artifacts. |
| D-016 | Cross-platform cutover evidence | Run the evidence recorder for Windows, Linux, macOS, Android, and iOS | One reviewed record exists per required platform; no platform is inferred from another. |

## Endpoint Registration Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| E-001 | Enrollment token | Create enroll-only token | Token has expiration/scope and can be revoked |
| E-002 | Register endpoint | Run registration script | Device appears or updates in dashboard |
| E-003 | Duplicate handling | Register same endpoint twice | Existing device updates, duplicate is not created |
| E-004 | Metadata | Register hostname/OS/version | Dashboard shows expected metadata |
| E-005 | Token revocation | Revoke token and retry registration | Registration fails with clear error |
| E-006 | Deploy endpoint compatibility | Call `/api/devices/deploy` with RustDesk-shaped body | Returns documented response and registers/updates expected device |
| E-007 | Deploy endpoint auth | Call `/api/devices/deploy` without bearer token | Request denied and no device is created |
| E-008 | Enrollment lifecycle evidence | Scope, use, revoke/expire, and retry a token on each required endpoint platform | Evidence shows lifecycle without recording token material. |

## Remote Session Workflow Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| R-001 | Copy ID | Copy RustDesk ID from dashboard | Clipboard contains correct ID |
| R-002 | Connect from operator | Connect to listed endpoint using official client | Session establishes through expected server |
| R-003 | Direct vs relay | Test LAN and WAN scenarios | Direct connection or relay behavior is documented and acceptable |
| R-004 | File transfer | Transfer small file | File arrives intact |
| R-005 | Unattended access | Connect to unattended test endpoint | Works according to endpoint password/security policy |
| R-006 | Connection helper | Generate default-server and explicit-server helpers | Helper output launches successfully or unsupported behavior is documented for the target OS/client version |

## Security Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| SEC-001 | HTTPS | Access admin UI over HTTP | Redirects or refuses plaintext access in production |
| SEC-002 | No secrets in logs | Run install/register flow | Logs contain no passwords or full secret tokens |
| SEC-003 | Auth required | Access device list unauthenticated | Request denied |
| SEC-004 | Token scope | Use enrollment token for admin API | Request denied |
| SEC-005 | Backup sensitivity | Inspect backup | Known sensitive values encrypted or explicitly documented |
| SEC-006 | CI secret scan | Run configured CI secret/privacy scan | No production secrets or site-specific values are detected outside ignored paths |
| SEC-007 | Misleading access-control claims | Review UI/API/docs for session enforcement wording | No UI or docs claim RustDesk session enforcement unless enforcement tests exist |
| SEC-008 | Role permission matrix | Attempt each role/action/site/tag case | Allowed and denied results match policy, with audit coverage for sensitive denials |
| SEC-009 | Optional passkeys | Register and use a phone passkey for OpenDesk login | Passkey login works, fallback/recovery is documented, and RustDesk session enforcement is not implied |

## CI Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| CI-001 | Docs workflow | Run Markdown/link/docs checks | Workflow passes on PR and `main` |
| CI-002 | Security workflow | Run secret and project-specific privacy scans | Workflow fails on seeded fake secret and passes on clean tree |
| CI-003 | Ignore boundary | Assert `local/` and `upstream/` are ignored | CI proves private/reference folders are excluded |
| CI-004 | Script validation | Run shellcheck/PowerShell validation for generated templates | Script templates pass static validation |
| CI-005 | Application tests | Run unit/integration tests once app exists | Required test suite passes |
| CI-006 | Container build | Build application container once app exists | Image builds reproducibly without production secrets |
| CI-007 | File size limits | Run source/document size report | Files over soft limits are absent or have documented justification |
| CI-008 | Canonical naming | Run naming/contract review check | No internal synonym/shim creep is introduced without boundary documentation |
| CI-009 | Public content hygiene | Run public content scan | Committed project content is free of private-workflow markers |

## Research Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| RS-001 | Client config behavior | Complete R-001 research matrix | Required OS/package/version config paths have supported/unsupported decisions and evidence |
| RS-002 | Deployment mechanics | Complete R-002 deployment research | Silent install, config location, service/user config, upgrade persistence, and ID/version readout are documented |
| RS-003 | Pro usage inventory | Complete R-003 inventory | Every Pro capability has Used Today, Replacement Path, Validation IDs, Evidence, and Blocker fields |
| RS-004 | Address book/password model | Complete R-004 research | Passwordless/managed-password dependency is known and secret-management decision is recorded |
| RS-005 | Access enforcement | Complete R-005 research | Dashboard RBAC vs session enforcement boundary has accepted implementation decision |
| RS-006 | Deploy endpoint compatibility | Complete R-006 research | Implement/defer/reject decision is backed by client behavior evidence |
| RS-007 | Session/audit log sources | Complete R-007 research | Audit capability matrix decides launch-intent, log ingestion, or deeper integration |
| RS-008 | Mobile workflow | Complete R-008 research | Mobile support level and validation scope are decided |
| RS-009 | Network behavior | Complete R-009 research | Ports, NAT, LAN, DNS, direct/relay behavior, and relay scaling decisions are documented |
| RS-010 | License posture | Complete R-010 research | Clean-room/fork/link/redistribution rules are recorded in an ADR |

## Migration Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| MIG-001 | Import schema | Parse sanitized external export | Version 1 parses only allowlisted fields and rejects unknown fields at every nested boundary. |
| MIG-002 | Import safety | Parse export containing credential hash, JWT, private key, or secret fields | Parse is rejected before deserialization and no report contains sensitive values. |
| MIG-003 | Reconciliation dry run | Compare export against an OpenDesk snapshot | Report is deterministic and report-only; duplicate identity/rustdesk_id values and ambiguous snapshot matches are blocked. |
| MIG-004 | Scope mapping | Reconcile external groups/scopes to sites | Name similarity alone never maps a scope; only explicit operator mapping evidence can produce a mapping. |

## Cutover Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| CUT-001 | Parallel run | Run dashboard beside existing RustDesk service | No impact to existing sessions |
| CUT-002 | Pilot group | Enroll 2-5 devices | Pilot devices manageable from dashboard |
| CUT-003 | Pro dependency review | Compare daily workflow against Pro | No blocker remains for selected workflow |
| CUT-004 | Rollback | Disable dashboard and use old workflow | Existing RustDesk access still works |
| CUT-005 | Evidence package | Review one redacted evidence record per required official client platform | Windows, Linux, macOS, Android, and iOS records are independently reviewed; no unsupported claim remains. |
| CUT-006 | Update rollback | Exercise documented client/control-plane rollback after a cutover-candidate update | Last-known-good workflow restores dashboard operations and client connectivity without private topology in public evidence. |
