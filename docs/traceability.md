# Traceability

This document maps replacement requirements to validation coverage and distinguishes current implementation/discovery evidence from still-required release validation. Current tests and authorized read-only discovery establish scope or behavior; they do not provide owner signoff, client parity, pilot acceptance, or cutover approval.

## Evidence State

| State | Meaning |
|---|---|
| `established` | Current-repository test, controlled probe, or authorized read-only discovery supports the statement. |
| `required` | An executed OS/version/operator/network, owner-decision, pilot, or cutover evidence packet is still needed. |
| `policy boundary` | The owner decision is explicit, but validation must ensure the implementation does not overclaim. |

Required client scope is Windows, Linux, and macOS endpoints plus Android and iOS operator workflows. RBAC is dashboard/API-only and does not enforce RustDesk sessions.

## Requirement Coverage

| Requirement | Validation coverage | State and remaining gate |
|---|---|---|
| PR-001 Web admin console | C-001 through C-010 | established app coverage for auth, devices, export, audit, health, users, and deployment; required operator evidence remains. |
| PR-002 Device inventory | C-002, C-003, C-004, C-005, C-006 | established CRUD/search/export tests; required parity/import/pilot evidence remains. |
| PR-003 Address-book workflow | C-013, C-014, R-001 through R-006, CUT-003 | established scoped personal/shared read and peer/tag mutation tests; required released-client pilot and required-platform operator evidence remain. |
| PR-004 Sites/tags/notes/archive | C-003, C-004, C-005 | established site/tag/note/archive tests; required replacement mapping for populated Pro group/address-book workflows remains. |
| PR-005 Client delivery | D-001 through D-016, E-008 | partial Linux/generated-flow evidence; required independently reviewed Windows/Linux/macOS endpoint and Android/iOS operator evidence remains. |
| PR-006 Endpoint self-registration | E-001 through E-008 | established token/API, generated multi-OS script, and audit tests; required required-platform lifecycle execution evidence remains. |
| PR-007 Server health | S-001 through S-005, S-010 through S-012, C-008 | established controlled probes, authenticated diagnostics, and app dashboard; required real-client, UDP exception, and cutover evidence remain. |
| PR-008 Backup/restore | S-006, S-007, SEC-005, CUT-004 | established JSON round-trip tests, admin protection, and sensitivity metadata; required fresh-instance drill remains. |
| PR-009 Audit logs | C-007, E-004, SEC-002 | established UI/export, enrollment/deployment audit, and redaction tests; populated source tables establish scope; required tier decision/release evidence remains. |
| PR-010 RBAC | C-009, C-012 through C-015, SEC-003, SEC-004, SEC-008 | established dashboard roles, default-deny client data scope, address-book permissions, and transport admission tests; real direct/relay session evidence remains. |
| PR-011 Access boundary clarity | SEC-007 | policy boundary: dashboard/API RBAC only; required UI/docs wording check remains. |
| PR-012 Official clients | C-011 through C-015, D-006, D-010 | established API and transport-admission contract tests; required signatures, updates, and released desktop/mobile direct/relay execution evidence remain. |
| PR-013 No Pro dependency | CUT-003 | required owner inventory and equivalent/retired workflow decisions. |
| PR-014 Mobile operator apps | D-011, D-012, R-002 | required Android and iOS operator validation. |
| PR-015 Migration dry-run reconciliation | MIG-001 through MIG-005 | established strict, report-only external-boundary parser, explicit approved-manifest preconditions, and reconciliation tests; required authorized export, mapping evidence, review, and any separately approved write-path design remain. |
| SR-001 Public repo privacy | SEC-006, CI-002 | established current scans; required cutover-candidate scan remains. |
| SR-002 Runtime secrets outside Git | SEC-006, CI-002 | established public scan boundary; required deployment/config review remains. |
| SR-003 Enrollment token protection | E-001, E-005, SEC-004 | established lifecycle/scope/permission tests; required release evidence remains. |
| SR-004 Secure API sessions | C-001, C-011, SEC-001, SEC-003 | established browser and official-client token lifecycle tests; required production HTTPS/session evidence remains. |
| SR-005 No plaintext unattended passwords | SEC-002, SEC-005 | established policy/redaction coverage; required managed-password decision remains. |
| SR-006 Generated scripts no long-lived secrets | D-001, D-003, SEC-002, CI-004 | established template coverage where tested; required static/release checks remain. |
| SR-007 Audit/log redaction | SEC-002 | established redaction tests; required production log review remains. |
| SR-008 HTTPS | SEC-001 | required production deployment validation. |
| SR-009 Backup sensitivity | SEC-005 | established metadata path; required protected-backup review remains. |
| SR-010 Session enforcement claims | R-005, SEC-007 | policy boundary: no session enforcement claim; required wording review remains. |
| SR-011 Optional passkeys | SEC-009 | optional dashboard/API hardening; required only if enabled. |
| OR-001 Repeatable deployment | S-008, CI-006, CUT-001 | partial Compose/app evidence; required clean deployment and parallel-run evidence remain. |
| OR-002 Dashboard failure does not break RustDesk | CUT-001, CUT-004 | required failure-mode and rollback evidence. |
| OR-003 LXC/Proxmox diagnostics | C-008, S-001 through S-007 | observed runtime and controlled checks establish scope; required runbook/operations evidence remains. |
| OR-004 Parallel run | CUT-001, CUT-002 | required pilot and no-impact evidence. |
| OR-005 Rollback | CUT-004 | required rollback drill or documented exception. |
| OR-006 Upgrade | S-009, CI-005, CI-006 | required populated-instance upgrade evidence. |
| OR-007 Upstream versions recorded | D-005, D-010 | required version/checksum record for each client packet. |
| OR-008 CI checks | CI-001 through CI-009 | established bootstrap checks; required mature application/container checks remain. |
| OR-009 Canonical contracts and anti-shim discipline | CI-008 plus engineering review | required implementation review and contract checks. |
| OR-010 Source file size limits | CI-007 plus code review | established docs check; required implementation report remains. |
| OR-011 Research roadmap completion | RS-001 through RS-010 | required; current rows remain partial except RS-010. |
| IR-001 Monitor OSS services | S-002, S-003, C-008 | observed official services and app health establish direction; required deployment evidence remains. |
| IR-002 Public key/fingerprint handling | S-005, SEC-006 | controlled fingerprint path established; required release evidence remains. |
| IR-003 Connection helpers | R-001, R-002, R-006 | established default/explicit helper generation and copy tests; required per-OS launch/open behavior remains. |
| IR-004 Client config per OS/version | D-001 through D-010 | required Windows/Linux/macOS version matrix. |
| IR-005 Compatibility endpoints | C-011 through C-015, E-006, E-007 | established isolated account/sync/address-book contracts and bounded transport admission; released-client direct/relay evidence remains required before parity acceptance. |
| IR-006 Released client behavior validation | RS-001, RS-002, D-001 through D-010 | required released-package validation; source inspection is insufficient. |
| CR-001 Pro feature mapping | CUT-003 | required owner/reviewer inventory signoff. |
| CR-002 Core validation evidence | Entire validation matrix | required before cutover. |
| CR-003 Backup/restore drill | S-006, S-007 | required fresh-instance drill. |
| CR-004 Pilot group | CUT-002, CUT-003 | required pilot report and issue disposition. |
| CR-005 Rollback | CUT-004 | required rollback validation. |
| CR-006 Privacy scan | SEC-006, CI-002 | established current scan; required release scan. |
| CR-007 CI green | CI-001 through CI-009 | required cutover-candidate status. |
| CR-008 Research roadmap complete | RS-001 through RS-010 | required accepted evidence and decisions for all rows. |

## Known Coverage Gaps

- Add UI-copy snapshot tests for SEC-007.
- Add site/tag visibility cases for SEC-008 if scoping ships.
- Add client version/checksum evidence packets and release records.
- Execute WAN/UDP probes, pilot, parallel-run, and cutover drills.
