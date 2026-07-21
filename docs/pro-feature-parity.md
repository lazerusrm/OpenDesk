# Pro Feature Parity Map

This document tracks RustDesk Server Pro-style capabilities and how OpenDesk intends to cover them without depending on RustDesk Pro infrastructure. The authorized read-only baseline confirms populated Pro data structures for peers, users, groups/mappings, address books/linked peers, and audit tables, and observes official `hbbs`/`hbbr` plus an admin web service. Those observations establish scope only; they do not prove weekly use, parity, or cutover acceptance.

Status categories:

- `Core`: required for full replacement.
- `Stage 2`: required if current production workflow depends on it, otherwise next wave.
- `Research`: possible, but requires validation against official clients or deeper integration.
- `Deferred`: out of scope until there is a concrete business need.

## Feature Map

| Capability | OpenDesk Approach | Status | Evidence state / remaining gate |
|---|---|---:|---|
| Self-hosted ID server | Use OSS `hbbs`; OpenDesk monitors/configures it. | Core | Runtime component observed; health and cutover validation remain. |
| Self-hosted relay server | Use OSS `hbbr`; OpenDesk monitors/configures it. | Core | Runtime component observed; client WAN/direct-vs-relay validation remains. |
| Web admin console | Build OpenDesk web UI. | Core | Dashboard/API tests exist; operator evidence remains. |
| Device inventory | OpenDesk database. | Core | Data model/tests exist; import and operator parity remain. |
| Address book | OpenDesk device list/bookmarks. | Core | Populated Pro address-book/peer structures establish scope; workflow pilot remains. |
| Tags/groups/sites | OpenDesk metadata. | Core | Current app tests cover metadata; replacement of Pro mappings needs owner inventory. |
| Device notes | OpenDesk metadata. | Core | Current app tests cover notes; pilot evidence remains. |
| Client config distribution | Generated scripts/download instructions. | Core | Linux evidence exists; Windows/Linux/macOS and mobile validation remains. |
| Download configured client | Serve generated per-OS install/config flows. | Core | Implemented flows are not release acceptance; OS/version evidence remains. |
| Filename-based client config | Generate `rustdesk-host=...` filename. | Stage 2 | Fallback requires per-OS/version validation. |
| Endpoint self-registration | OpenDesk enrollment API/script with scoped, revocable tokens. | Core | Automated lifecycle coverage exists; endpoint matrix remains. |
| `/api/devices/deploy` compatibility | Implement RustDesk-shaped endpoint only if client evidence warrants it. | Research | Linux controlled validation exists; Windows/macOS and adapter evidence remain. |
| Audit log | OpenDesk audit events. | Core | Populated source audit structures establish requirement; tier decision and validation remain. |
| Session audit | Client/server log ingestion if available. | Research | Source visibility observed; owner must choose tier; no session-enforcement claim. |
| Access control | OpenDesk dashboard/API roles. | Core | Owner decision is dashboard/API RBAC only; permission tests exist; RustDesk sessions are out of scope. |
| RustDesk session ACLs | Endpoint/network/client integration. | Research | Not selected; never claim enforcement without Tier 3 evidence. |
| Central settings/policies | Generated config + endpoint registration service. | Core | Strategy/config rows establish scope; policy equivalence and OS validation remain. |
| Disable public server fallback | Scripted config where supported. | Core | Must validate released clients; hard enforcement may require separate decision. |
| Managed unattended passwords | External secret manager integration. | Research | Hashed secret material observed; owner must require, equate, or retire. |
| SSO/OIDC | Reverse proxy or app-native OIDC. | Stage 2 | No inspected third-party auth dependency established; owner review remains. |
| LDAP | External IdP integration. | Deferred | No current dependency established. |
| 2FA | Reverse proxy/IdP first. | Stage 2 | No inspected enabled 2FA dependency; optional hardening. |
| Passkeys | App-native WebAuthn/passkey support for OpenDesk login. | Stage 2 | Optional dashboard/API hardening; not RustDesk session enforcement. |
| Custom client builder | Generated scripts/wrapper. | Research | Windows-only custom-client records observed; owner must classify current use. |
| Branding | OpenDesk web UI branding. | Core | UI exists; owner parity review remains. |
| Native RustDesk address book | Client fork or compatible API. | Research | Not proven necessary; web equivalent requires pilot acceptance. |
| Browser/web remote client | Do not build initially. | Deferred | Out of scope. |
| Mobile app operator workflow | Generated manual/QR config instructions. | Core | Android/iOS are required operator validations before cutover; not managed endpoint scope. |
| Backups | OpenDesk backup/restore. | Core | Automated round-trip tests exist; fresh-instance drill remains. |
| Health checks | DNS/port/service/key fingerprint checks. | Core | Controlled baseline/runtime evidence exists; real-client and cutover checks remain. |

## Required Usage Inventory

For every capability above, the owner must record `Used Today` as `yes`, `no`, `unknown`, or `retired by owner decision`; replacement path; validation IDs; evidence; and blocker. Populated database rows are evidence that a structure exists, not `Used Today: yes` and not acceptance. Any unknown value blocks cutover. Any yes value needs passing validation or a signed retirement decision.

## Required Client Scope

Before cutover, validate official clients on Windows, Linux, and macOS, and validate Android and iOS operator workflows. Preserve official-client delivery and OSS `hbbs`/`hbbr`; do not infer released-client behavior from source inspection alone.

## Replacement Acceptance

OpenDesk can be considered a Pro replacement only after the owner inventory is complete, required workflows pass, all research rows are accepted or explicitly retired, and cutover signoffs are recorded. Current implementation tests and authorized discovery are not those signoffs.

Acceptable only with explicit owner confirmation that they are not required: native in-app address book, RustDesk session-level ACL enforcement, managed password injection, browser remote desktop, custom signed clients, and drop-in Pro API compatibility.
