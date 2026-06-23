# Client Validation Procedures

These procedures close the remaining client research gaps with repeatable evidence. Use disposable test endpoints only.

## Evidence To Record

For each run, record:

- Date.
- Tester.
- OS and version.
- RustDesk client version and package type.
- Server version or source.
- Exact command or script used.
- Config values redacted to placeholders.
- Pass/fail for each step.
- Artifact path under ignored `local/research/`.

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

## Android

Target:

- Current official Android client if mobile operator workflow is required.

Steps:

1. Configure ID/relay/key manually.
2. Configure using QR payload with test-only host/key values.
3. Connect from mobile to a test endpoint.
4. Decide whether mobile is operator-only or managed endpoint.

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
