# Cutover Readiness

Production cutover is blocked until this checklist passes. Development can proceed in stages, but this gate defines replacement completeness.

## Documentation Baseline

- [ ] Requirements, architecture, parity map, validation matrix, traceability, threat model, CI plan, and engineering standards have been reviewed together.
- [ ] Traceability has no missing or stale requirement/validation references.
- [ ] The Pro usage inventory in `docs/pro-feature-parity.md` has no `unknown` entries.
- [ ] Validation evidence rules in `docs/validation-matrix.md` are understood before implementation tasks are opened.
- [ ] An independent high-rigor reviewer signs off the documentation baseline before implementation starts.

## Prerequisites

- [ ] Current production Pro usage has been inventoried.
- [ ] Every used Pro feature is mapped in `docs/pro-feature-parity.md`.
- [ ] Every Core requirement in `docs/requirements.md` has validation evidence.
- [ ] Required endpoint OSes are identified.
- [ ] Required operator workflows are identified.
- [ ] Real deployment details remain in ignored local/runtime config only.

## Parallel Run

- [ ] OpenDesk runs beside current RustDesk service without disrupting sessions.
- [ ] Pilot devices are enrolled.
- [ ] Pilot operators use OpenDesk for normal lookup/connect workflows.
- [ ] Issues from pilot are tracked and closed or explicitly accepted.
- [ ] RustDesk Pro remains available during pilot.

## Technical Gates

- [ ] Admin auth and RBAC pass.
- [ ] Device inventory and address book workflows pass.
- [ ] Client delivery passes for required OSes.
- [ ] Endpoint enrollment and check-in pass.
- [ ] Health checks pass.
- [ ] Backup and restore drill passes.
- [ ] Audit logs capture required events.
- [ ] Privacy scan passes outside ignored paths.
- [ ] Required CI workflows are green on the cutover candidate commit.
- [ ] Generated scripts contain no long-lived privileged secrets.
- [ ] Official RustDesk installer signatures/checksums are validated where applicable.

## Security Gates

- [ ] HTTPS is enforced for production admin UI/API.
- [ ] Enrollment token lifecycle passes.
- [ ] Logs and audit events redact secrets.
- [ ] Backup sensitivity is documented.
- [ ] Unattended password handling is either out of scope for production or implemented through an approved secret-management design.
- [ ] Any access enforcement claim has matching enforcement evidence.

## Rollback

- [ ] Existing RustDesk access remains available during cutover.
- [ ] DNS/proxy changes are documented.
- [ ] OpenDesk can be stopped without breaking `hbbs`/`hbbr`.
- [ ] Last known-good backup is available.
- [ ] Operators know the fallback workflow.

## Signoff

Required signoffs:

- [ ] Owner accepts feature parity mapping.
- [ ] Independent high-rigor reviewer accepts all public planning, architecture, parity, validation, threat-model, CI, traceability, and standards docs.
- [ ] Technical reviewer accepts architecture and validation evidence.
- [ ] Security/privacy scan is clean.
- [ ] Pilot operator workflow is accepted.

Cutover decision:

- [ ] Approved
- [ ] Rejected
- [ ] Deferred pending listed blockers
