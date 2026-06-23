# Tasks

## Immediate Next Steps

- [ ] Create Git repository and remote.
- [ ] Decide first implementation stack: Go server-rendered UI vs FastAPI + React.
- [ ] Inventory current RustDesk LXC configuration.
- [ ] Export current RustDesk public key and document client config.
- [ ] Define required day-one OS targets.
- [ ] Decide whether the first deployment runs in the existing LXC or a new LXC.
- [x] Clone upstream RustDesk client/server locally for reference.
- [x] Document upstream hooks relevant to OpenDesk.
- [x] Create Pro feature parity map.
- [x] Add replacement requirements.
- [x] Add architecture and threat model docs.
- [x] Add cutover readiness gate.
- [x] Add CI plan and CI validation requirements.
- [x] Add bootstrap docs/security workflows.
- [x] Add requirements-to-validation traceability.
- [x] Add engineering standards for canonical contracts, anti-shim policy, naming, and file size limits.
- [x] Add public content hygiene scan and ignore rules for private local files/folders.
- [x] Add explicit research roadmap for remaining RustDesk replacement unknowns.

## Research Sprint

- [ ] Fill R-001 client configuration behavior matrix.
- [ ] Fill R-002 official client deployment mechanics.
- [ ] Fill R-003 production Pro usage inventory.
- [ ] Fill R-004 address book/password model.
- [ ] Fill R-005 access control reality.
- [ ] Fill R-006 deployment endpoint compatibility.
- [ ] Fill R-007 session/audit log sources.
- [ ] Fill R-008 mobile workflow.
- [ ] Fill R-009 relay/NAT/LAN/DNS behavior.
- [ ] Fill R-010 legal/license posture.

## First Build Slice

- [ ] Scaffold backend service.
- [ ] Add SQLite migrations.
- [ ] Add admin login.
- [ ] Add device CRUD.
- [ ] Add server config settings page.
- [ ] Add Windows install/config script generator.
- [ ] Add Linux install/config script generator.
- [ ] Add endpoint enrollment token model.
- [ ] Add endpoint registration endpoint.

## Questions For Owner

- Which RustDesk Pro features are used every week today?
- Are passwords stored in the RustDesk Pro address book today?
- Which endpoints matter first: Windows desktops, Linux desktops, servers, macOS, mobile?
- Is there already a reverse proxy/auth stack for `example.com` services?
- Do we want `rd-admin.example.com` as the dashboard hostname?
