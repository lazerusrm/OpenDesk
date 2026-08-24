# Client Validation Procedures

These procedures close the remaining client research gaps with repeatable evidence. Use disposable test endpoints only.

## Initial Cutover Run Order

Run one complete record for each official client platform: Windows, Linux, macOS,
Android, and iOS. Do not substitute another OS, package, architecture, or client
version. Android and iOS are operator validation unless a separate endpoint decision
is approved.

For each platform, use this order:

1. Capture the release source, exact version/package, and signature/checksum result.
2. Capture server configuration readback and public-key fingerprint using test values.
3. Enroll/check in only where endpoint enrollment is in scope; prove token scope and revocation.
4. Test OpenDesk dashboard/API RBAC with permitted and denied test roles. This is not RustDesk session enforcement.
5. Exercise LAN, WAN, NAT, and relay fallback as separate transport cases; redact topology.
6. Repeat config readback after restart and official update; then exercise the documented rollback.
7. Have an independent reviewer mark the record final and link artifacts under ignored `local/research/`.

Create a status-only checklist without secrets or command output:

```bash
scripts/validation-evidence-record.sh --case cutover-windows --platform windows \
  --client-version '<official-version>' --package installer \
  --enrollment not-run --server-config not-run --access not-run \
  --wan not-run --nat-relay not-run --update not-run --rollback not-run
```

The same command is used with `linux`, `macos`, `android`, and `ios`. Replace statuses
only after evidence review. Never pass tokens, keys, passwords, hostnames, addresses,
or endpoint identifiers to the recorder.

The recorder validates all free-form metadata before writing it: case names and client
versions are bounded ASCII identifiers/semantic versions, package labels are an allowlist,
and topology/credential markers, URLs, IP-like values, whitespace, and control characters
are rejected. Run its negative-input smoke tests before using it:

```bash
bash scripts/test-validation-evidence-record.sh
```


## Evidence To Record

For each run, record:

- Date.
- Tester and independent reviewer.
- OS/architecture and version.
- RustDesk client version and package type.
- Server version or source.
- Exact command or script used (with credentials omitted).
- Config values redacted to placeholders and key fingerprint only.
- Enrollment, server config, dashboard RBAC, WAN, NAT/relay, update, and rollback status.
- Pass/fail for each step and exception owner/expiry if applicable.
- Artifact path under ignored `local/research/`.

Use `scripts/research-client-record.sh` to create the starting evidence file for a client run. On Windows or macOS, prefer the platform capture scripts:

- `scripts/research-windows-client-record.ps1`
- `scripts/research-macos-client-record.sh`

## Windows Installer

Target:

- Current official Windows x64 installer.
- Windows 10 or Windows 11 test endpoint.

Steps:

1. Verify downloaded installer signature/checksum against the official release source.
2. Run silent install.
3. Apply config with `--config` using test-only ID server, relay server, API server, and key.
4. Read back ID with `--get-id`.
5. Read back configured options where supported.
6. Restart the RustDesk service.
7. Confirm configured server values persist.
8. Reinstall or upgrade to the same/current version.
9. Confirm configured server values and ID persist.
10. Run `--deploy --token` against a dev capture endpoint.

Passing criteria:

- Installation requires no executable renaming.
- Config values persist for the service context.
- ID readout works.
- Deploy request shape matches the Linux evidence or differences are documented.

Evidence helper:

```powershell
./scripts/research-windows-client-record.ps1 -RustDeskPath "C:\Program Files\RustDesk\rustdesk.exe" -ConfigString "<test-config>" -DeployToken "<test-token>"
```

Deploy capture helper:

```bash
scripts/research-deploy-capture-server.py --bind 0.0.0.0 --port 18080 --result OK --output local/research/manual/windows-deploy-capture.jsonl
```

## Windows Portable

Target:

- Current official Windows x64 portable executable.

Steps:

1. Run portable executable with filename-based `host=`, `key=`, and `relay=` config.
2. Repeat with a browser-style duplicate suffix in the filename.
3. Test `--config` if the portable package accepts it without install.
4. Test elevation path if unattended support requires admin-level UI access.

Passing criteria:

- Filename config is either reliable enough for fallback use or explicitly rejected.
- Duplicate filename behavior is documented.
- Portable path is not the primary managed deployment unless service/config persistence is proven.

## macOS

Target:

- Current official `.dmg`.
- Intel and Apple Silicon if both are required.

Steps:

1. Install from `.dmg` into Applications.
2. Grant required screen/input permissions on a test device.
3. Apply config with supported command/import/manual path.
4. Confirm user and service/root config behavior.
5. Restart service/app and confirm persistence.
6. Upgrade/reinstall and confirm persistence.
7. Run deploy endpoint test if the command is available.

Passing criteria:

- Required permissions are documented.
- Config persistence is proven for the context that accepts incoming sessions.
- Any manual-only step is marked as a cutover constraint.

Evidence helper:

```bash
scripts/research-macos-client-record.sh --rustdesk /Applications/RustDesk.app/Contents/MacOS/RustDesk --config '<test-config>' --deploy-token '<test-token>'
```

Deploy capture helper:

```bash
scripts/research-deploy-capture-server.py --bind 0.0.0.0 --port 18080 --result OK --output local/research/manual/macos-deploy-capture.jsonl
```

## Android

Target:

- Current official Android client if mobile operator workflow is required.

Steps:

1. Configure ID/relay/key manually.
2. Configure using QR payload with test-only host/key values.
3. Connect from mobile to a test endpoint.
4. Decide whether mobile is operator-only or managed endpoint.

Evidence helper:

```bash
scripts/research-mobile-config-record.sh --case android-operator --host '<test-host>' --key '<test-public-key>'
```

Passing criteria:

- Manual/QR setup is documented well enough for operators.
- Managed mobile endpoint support is either validated or explicitly out of scope.

## iOS

Target:

- Current official iOS client if mobile operator workflow is required.

Steps:

1. Configure self-hosted server manually if supported.
2. Connect from iOS to a test endpoint.
3. Confirm no requirement depends on controlling iOS as an endpoint.

Passing criteria:

- iOS is documented as operator-only unless official capability changes.
- Any unsupported workflow is retired or assigned a separate future design.
