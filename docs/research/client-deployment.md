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

Current finding:

Official documentation and local source inspection both support command-line configuration. The config command parses the same custom-server string used by filename configuration and writes server key, ID server, API server, and relay server options.

Preferred validation environment:

Use a separate dev LXC or VM for endpoint/package validation before touching production. The dev environment should be disposable, have known snapshots, and run only test RustDesk IDs/keys.

Decision:

Use generated scripts as the primary path and filename configuration only as a fallback/convenience path until each required released client proves reliable behavior.

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

Current finding:

Local source inspection confirms official commands for silent install, service install, config import, config string application, ID readout, ID set, password set, assignment, and deployment. These commands generally require an installed client and admin/root permissions when changing local service settings.

Service-context finding:

Windows service creation imports a config file into the service path. macOS privileged install copies user preference files into the root preference location before loading the service. Validation must prove both interactive user context and service/system context are configured.

Preferred validation environment:

Use disposable dev LXC/VM targets for Linux package tests and separate Windows/macOS VMs for service-context tests. Record install, configure, restart, upgrade, uninstall, and reinstall results before cutover.

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

Current finding:

Local source inspection confirms a scriptable deploy command that posts to the configured API server with a bearer token and can return deployment-specific errors. This supports prototyping an isolated compatibility adapter, but not coupling the main domain model to that API shape.

Preferred validation environment:

Stand up the compatibility adapter in a dev LXC first. Test stock clients against the dev API server, record request/response shape, and only then decide whether production OpenDesk should expose the endpoint.

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
