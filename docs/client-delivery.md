# Client Delivery Plan

## Objective

Let an admin download or run a client installer/config flow from our frontend without relying on renaming the RustDesk executable.

## Constraints

- Prefer official RustDesk client binaries so RustDesk continues to own signing, notarization, mobile distribution, and updater behavior.
- Do not modify official binaries during the initial control-plane build.
- Do not embed secrets in public download URLs.
- Do not store unattended passwords in generated scripts.
- Keep server config reproducible and visible in the dashboard.

## Delivery Options

### Option 0: Generated Filename Config

Current upstream RustDesk source contains a custom server parser for executable names like:

```text
rustdesk-host=server.example.net,key=PUBLIC_KEY,relay=server.example.net.exe
```

OpenDesk can generate this filename and download instruction as a compatibility path, but this must not be the only replacement path because the user experience is brittle and OS/package dependent.

Validation required:

- Works on the current official Windows build.
- Behavior is clear for duplicate filenames such as `(1)`.
- Does not break installer signatures beyond normal filename changes.
- Does not become the required primary workflow.

### Option 1: Scripted Official Install

The frontend generates a per-OS install/config script.

Windows:

- Download official RustDesk installer.
- Install silently where supported.
- Apply server config through supported client config/import mechanism.
- Register endpoint with our control plane.

Linux:

- Download official package or use distro package if available.
- Apply server config.
- Enable/start service if appropriate.
- Register endpoint.

macOS:

- Download official DMG/pkg.
- Provide guided install or script if permissions allow.
- Apply server config where feasible.
- Register endpoint.

This is the recommended first implementation path, but cutover requires this to be validated for every required endpoint OS.

### Option 1A: Official Deploy Endpoint Compatibility

Current upstream client source includes a deployment call shaped like:

```text
POST /api/devices/deploy
Authorization: Bearer <token>
{"id":"...","uuid":"...","pk":"..."}
```

OpenDesk may implement this endpoint if current official clients can call it without RustDesk Server Pro. Treat it as an optional compatibility path, not the primary registration path, until validated.

### Option 2: Cached Official Installer Plus Config Script

The frontend serves a known tested official installer version from our server plus a matching script.

Pros:

- Repeatable.
- Avoids upstream download outages.
- Lets us validate a known version.

Cons:

- We must track upstream releases and security fixes.
- We must preserve original signatures and checksums.

### Option 3: Custom Wrapper Installer

Build our own small wrapper installer that downloads/runs official RustDesk and applies config.

Pros:

- Cleaner user experience.
- Can bundle enrollment flow.

Cons:

- We own signing/trust for the wrapper.
- Adds packaging work.

### Option 4: Light RustDesk Fork

Build custom clients with default server config embedded.

Pros:

- Best UX.
- Can add dashboard link or native address book later.

Cons:

- We own desktop signing, packaging, upstream merges, and support.
- Deferred until workflow proves it needs this.

## Initial Cutover Scope

The initial cutover validation set is fixed: official RustDesk clients on Windows,
Linux, macOS, Android, and iOS. Desktop clients are endpoint candidates; Android and
iOS are operator workflows unless a separate endpoint decision is approved. A result
from one platform or package must never be used as evidence for another.

For every platform, capture the exact official release, package/source, checksum or
signature result where applicable, server configuration readback, enrollment/check-in,
and an independent review. Use only test hosts, public-key fingerprints, and redacted
labels in committed summaries. Raw screenshots and logs stay under ignored
`local/research/`.

The evidence package must separately record:

- Enrollment lifecycle: scope, expiration/revocation, check-in, and retry behavior.
- Server config: ID/relay/API values represented by placeholders and public-key fingerprint.
- Access expectation: OpenDesk dashboard/API authentication and RBAC only; never session enforcement.
- Transport: LAN, WAN, NAT traversal, and relay fallback as separate observations.
- Update: official source/checksum and before/after client identity/config readback.
- Rollback: last-known-good client/control-plane procedure and restoration result.

`scripts/validation-evidence-record.sh` creates a status-only, redacted checklist for
this package. It intentionally accepts no secret-bearing values or command output.

## Download Page

Fields:

- OS selector.
- RustDesk version.
- Server config summary.
- Public key fingerprint.
- Enrollment token scope/expiration.
- Download script button.
- Copy install command button.
- Checksum for downloaded official installer if cached.

## Passing Criteria

- Every required platform has an independently reviewed record: Windows, Linux, macOS, Android, and iOS.
- New Windows endpoint can be installed/configured without executable renaming.
- New Linux endpoint can be installed/configured without manual RustDesk server entry.
- Endpoint appears in our dashboard after install where endpoint enrollment is in scope.
- RustDesk transport result is recorded for the tested path; it is not an access-enforcement claim.
- Official client signatures/checksums remain valid.
- Update persistence and rollback are exercised, not inferred.
- If filename-based config is offered, it is a fallback or convenience path, not the only supported path.
