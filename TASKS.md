# Tasks

## Immediate Next Steps

- [x] Public self-host install guide (`docs/install.md`, README entry).
- [x] OSS self-host first-run (goal below).
- [ ] Create Git repository and remote.
- [x] Decide first implementation stack: Rust backend with server-rendered UI first.
- [x] Inventory current RustDesk LXC configuration.
- [x] Export current RustDesk public key and document client config privately.
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

- [~] Fill R-001 client configuration behavior matrix.
- [~] Fill R-002 official client deployment mechanics.
- [~] Fill R-003 production Pro usage inventory.
- [~] Fill R-004 address book/password model.
- [~] Fill R-005 access control reality.
- [~] Fill R-006 deployment endpoint compatibility.
- [~] Fill R-007 session/audit log sources.
- [~] Fill R-008 mobile workflow.
- [~] Fill R-009 relay/NAT/LAN/DNS behavior.
- [x] Fill R-010 legal/license posture.

## First Build Slice

- [x] Scaffold backend service.
- [x] Add SQLite migrations.
- [x] Add admin login.
- [x] Add device CRUD.
- [x] Add server config settings page.
- [x] Add Windows install/config script generator.
- [x] Add Linux install/config script generator.
- [x] Add endpoint enrollment token model.
- [x] Add endpoint registration endpoint.

## Goal: OSS self-host first-run

Make a fresh OpenDesk checkout usable by a third party who does not have
organization signing keys.

Build:

- First-run wizard when the database has no users. Username is always `admin`.
  The operator sets the password and the public base URL, ID server, relay,
  API URL, and hbbs public key. Do not create `admin` from an env password
  when the wizard is enabled.
- If `OPENDESK_CLIENT_TOKEN_HMAC_KEY` is unset on first start, generate at
  least 32 random bytes, persist next to the SQLite file, and reuse that file
  on later starts. Refuse to start if an existing database has no stored key
  and the env is also unset.
- Keep organization-signed Windows installers opt-in through
  `OPENDESK_SIGNED_CLIENT_DIR`. Default UI is official RustDesk downloads plus
  generated scripts.
- Keep `hbbs`/`hbbr` as official OSS binaries. Do not vendor AGPL RustDesk.

Test:

- Empty data directory: wizard, then login as `admin` with the chosen
  password; env bootstrap password is not required.
- Signed directory unset: no `setup.exe` links; PowerShell script still
  renders.
- Signed directory with `windows-setup.exe`: download appears.
- HMAC key file is created once and reused across process restart.
- `scripts/docs-check.sh`, `scripts/privacy-scan.sh`, and
  `scripts/public-content-scan.sh` stay green. No production hosts or secrets
  in the repository.

Validate:

- Follow `docs/install.md` on a clean machine (Compose or `cargo run`).
- Official client can import ID, relay, API, and key from Settings.
- Loopback bind plus reverse-proxy TLS remains the documented production
  posture. Publishing port 8080 stays lab-only.

Out of scope: OIDC, LDAP, SMTP, vault unattended passwords, vendoring AGPL
RustDesk, in-process Authenticode, RustDesk session ACL enforcement.

## Questions For Owner

- Which RustDesk Pro features are used every week today?
- Are passwords stored in the RustDesk Pro address book today?
- Which endpoints matter first: Windows desktops, Linux desktops, servers, macOS, mobile?
- Is there already a reverse proxy/auth stack for `example.com` services?
- Do we want `rd-admin.example.com` as the dashboard hostname?
