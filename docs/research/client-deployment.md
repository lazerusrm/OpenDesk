# Client And Deployment Research

## R-001 Client Configuration Behavior By OS And Version

Question:

Which official RustDesk client configuration mechanisms work reliably for each required OS/package/version?

Must cover:

- Windows installer.
- Windows portable executable.
- Linux `.deb`/`.rpm` where applicable.
- Linux AppImage or flatpak if used.
- macOS Intel and Apple Silicon packages if used.
- Android and iOS configuration limits if mobile operator workflow matters.

Configuration methods to test:

- Manual network settings.
- Import/export config.
- Automatic config behavior from official docs.
- Command-line `--config`.
- Filename-based `host=`, `key=`, `relay=` behavior.
- Explicit connection helper strings such as `DEVICE_ID@server:port?key=...`.

Output:

- A filled OS/package/version matrix.
- Clear decision on the primary configuration path per OS.
- Clear decision on whether filename-based config is supported, fallback-only, or rejected.

Related validation:

- D-001 through D-010.
- R-006.
- RS-001.

## R-002 Official Client Deployment Mechanics

Question:

How exactly do we install, configure, upgrade, and verify official RustDesk clients without Pro custom clients?

Must cover:

- Silent install flags per OS/package.
- Config storage location per OS.
- Difference between user config and service/system config, especially Windows.
- Whether configured server settings survive upgrade/reinstall.
- Whether service restart is required after config changes.
- How to read the local RustDesk ID and client version safely.
- How to avoid executable renaming as the primary workflow.

Output:

- Per-OS deployment procedure.
- Script template requirements.
- Upgrade survivability evidence.

Related validation:

- D-001 through D-010.
- S-009.
- RS-002.

## R-006 Deployment Endpoint Compatibility

Question:

Can stock official clients use an OpenDesk implementation of the RustDesk-shaped deploy endpoint without Pro assumptions?

Must determine:

- How clients discover/use `api-server`.
- Whether the deploy action is exposed in UI or scriptable.
- Exact request/response schema required by current clients.
- Behavior when deployment changes local ID.
- Error handling for duplicate IDs, invalid token, and offline API.
- Whether implementing the endpoint creates unwanted coupling to RustDesk Pro behavior.

Output:

- Compatibility decision: implement, defer, or reject.
- If implemented, isolated endpoint contract and tests.

Related validation:

- E-006.
- E-007.
- RS-006.

## R-008 Mobile Workflow

Question:

What does full replacement require for iOS/Android operators or endpoints?

Must determine:

- Whether mobile is operator-only or also managed endpoint.
- How official mobile clients are configured for self-hosted servers.
- Whether mobile configuration can be automated.
- Whether address-book workflow from OpenDesk browser is sufficient.
- Whether mobile limitations block cutover.

Output:

- Mobile support decision.
- Mobile validation rows expanded if mobile is required.

Related validation:

- D-series client delivery tests.
- R-series connection workflow tests.
- RS-008.

