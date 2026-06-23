# Validation Matrix

## Environments

Target test environments:

- Windows 10 or 11 endpoint
- Linux desktop endpoint
- Linux server/LXC environment for control plane
- Optional macOS endpoint
- Optional Android/iOS operator client, configuration-only

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

## Remote Session Workflow Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| R-001 | Copy ID | Copy RustDesk ID from dashboard | Clipboard contains correct ID |
| R-002 | Connect from operator | Connect to listed endpoint using official client | Session establishes through expected server |
| R-003 | Direct vs relay | Test LAN and WAN scenarios | Direct connection or relay behavior is documented and acceptable |
| R-004 | File transfer | Transfer small file | File arrives intact |
| R-005 | Unattended access | Connect to unattended test endpoint | Works according to endpoint password/security policy |

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

## Cutover Validation

| ID | Function | Test | Passing Criteria |
|---|---|---|---|
| CUT-001 | Parallel run | Run dashboard beside existing RustDesk service | No impact to existing sessions |
| CUT-002 | Pilot group | Enroll 2-5 devices | Pilot devices manageable from dashboard |
| CUT-003 | Pro dependency review | Compare daily workflow against Pro | No blocker remains for selected workflow |
| CUT-004 | Rollback | Disable dashboard and use old workflow | Existing RustDesk access still works |
