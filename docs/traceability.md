# Traceability

This document maps replacement requirements to validation coverage. It should be updated whenever requirements or validation cases change.

## Requirement Coverage

| Requirement | Validation Coverage | Notes |
|---|---|---|
| PR-001 Web admin console | C-001 through C-010 | Covers auth, device workflows, export, audit, health, user admin, and deployment UI. |
| PR-002 Device inventory | C-002, C-003, C-004, C-005, C-006 | Device CSV export at `/devices/export.csv` covered by `device_csv` unit tests and `device_csv_export_integration` tests; respects default-list archive exclusion and current search term. |
| PR-003 Address-book workflow | R-001 through R-006, CUT-003 | RustDesk ID copy button on device list and edit form covered by `rustdesk_id_copy_integration` tests (R-001 copy ID validation). Pilot workflow evidence still required for cutover. |
| PR-004 Sites/tags/notes/archive | C-003, C-004, C-005 | Site CRUD + device assignment covered by sites_integration tests; tag CRUD + device tag assignment + tag search covered by tags_integration tests; notes edit persistence covered by integration_test and notes list display/search covered by notes_integration tests; default list excludes archived devices via `devices_for_default_list` unit tests, `devices_list_contract` integration test, and launch seed archive check. |
| PR-005 Client delivery | D-001 through D-010 | Must pass for required OSes before cutover. |
| PR-006 Endpoint self-registration | E-001 through E-007 | E-002/E-003/E-004 covered by enrollment API, generated Linux script, and integration tests. |
| PR-007 Server health | S-001 through S-005, C-008 | UDP reachability may need documented exception. |
| PR-008 Backup/restore | S-006, S-007, SEC-005, CUT-004 | Restore drill required. |
| PR-009 Audit logs | C-007, E-004, SEC-002 | Add event-specific tests with implementation. |
| PR-010 RBAC | C-009, SEC-003, SEC-004, SEC-008 | Role matrix must cover admin/operator/read-only behavior before cutover. |
| PR-011 Access boundary clarity | SEC-007 | Docs currently state boundary; implementation must avoid misleading UI. |
| PR-012 Official clients | D-006, D-010 | Signature/checksum and update behavior. |
| PR-013 No Pro dependency | CUT-003 | Pro usage inventory from CR-001 remains required for signoff. |
| PR-014 Mobile operator apps | D-011, D-012, R-002 | Required mobile RustDesk app operator workflow. |
| SR-001 Public repo privacy | SEC-006, CI-002 | Bootstrap privacy scan exists. |
| SR-002 Runtime secrets outside Git | SEC-006, CI-002 | Add config review once app exists. |
| SR-003 Enrollment token protection | E-001, E-005, SEC-004 | Add storage/hash tests with implementation. |
| SR-004 Secure API sessions | C-001, SEC-001, SEC-003 | Add CSRF/session tests with implementation. |
| SR-005 No plaintext unattended passwords | SEC-002, SEC-005 | Add schema/code checks with implementation; ADR-008 governs managed access secrets. |
| SR-006 Generated scripts no long-lived secrets | D-001, D-003, SEC-002, CI-004 | Script templates must be statically checked. |
| SR-007 Audit/log redaction | SEC-002 | Add seeded secret redaction tests. |
| SR-008 HTTPS | SEC-001 | Production deploy validation. |
| SR-009 Backup sensitivity | SEC-005 | Backup classification required. |
| SR-010 Session enforcement claims | R-005, SEC-007 | Must remain non-enforcing unless proven. |
| SR-011 Optional passkeys | SEC-009 | OpenDesk auth hardening only; not session enforcement. |
| OR-001 Repeatable deployment | S-008, CI-006, CUT-001 | Compose smoke test once app exists. |
| OR-002 Dashboard failure does not break RustDesk | CUT-001, CUT-004 | Failure-mode test required. |
| OR-003 LXC/Proxmox diagnostics | C-008, S-001 through S-007 | Runbook coverage needed later. |
| OR-004 Parallel run | CUT-001, CUT-002 | Pilot group evidence. |
| OR-005 Rollback | CUT-004 | Rollback drill or documented no-op. |
| OR-006 Upgrade | S-009, CI-005, CI-006 | Add migration/upgrade tests with implementation. |
| OR-007 Upstream versions recorded | D-005, D-010 | Validation evidence must record versions. |
| OR-008 CI checks | CI-001 through CI-009 | Bootstrap docs/security checks exist; app-specific checks arrive with implementation. |
| OR-009 Canonical contracts and anti-shim discipline | CI-008 plus engineering review | Needs automated and human review once implementation begins. |
| OR-010 Source file size limits | CI-007 plus code review | Needs automated report once implementation begins. |
| OR-011 Research roadmap completion | RS-001 through RS-010 | Required before production cutover. |
| IR-001 Monitor OSS services | S-002, S-003, C-008 | Do not modify `hbbs`/`hbbr` initially. |
| IR-002 Public key/fingerprint handling | S-005, SEC-006 | Real key material remains ignored/private. |
| IR-003 Connection helpers | R-001, R-002, R-006 | Validate copy/open behavior for default and explicit server cases. |
| IR-004 Client config per OS/version | D-001 through D-010 | Matrix must record OS/package/version. |
| IR-005 Compatibility endpoints | E-006, E-007 | Only if implemented. |
| IR-006 Released client behavior validation | RS-001, RS-002, D-001 through D-010 | Do not rely on source hooks without package validation. |
| CR-001 Pro feature mapping | CUT-003 | Owner/reviewer signoff. |
| CR-002 Core validation evidence | Entire validation matrix | Required before cutover. |
| CR-003 Backup/restore drill | S-006, S-007 | Required before cutover. |
| CR-004 Pilot group | CUT-002, CUT-003 | Required before cutover. |
| CR-005 Rollback | CUT-004 | Required before cutover. |
| CR-006 Privacy scan | SEC-006, CI-002 | Required before release/cutover. |
| CR-007 CI green | CI-001 through CI-009 | Required on cutover candidate commit. |
| CR-008 Research roadmap complete | RS-001 through RS-010 | Required on cutover candidate. |

## Known Coverage Gaps

- Expand SEC-007 into UI-copy snapshot tests once UI exists.
- Expand SEC-008 into a full role/action/site/tag permission matrix once RBAC exists.
- Add generated script fixture tests once templates exist.
- Add migration/backup/restore automation once the app exists.
- Add release evidence format for client versions and checksums.
