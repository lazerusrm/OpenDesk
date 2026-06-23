# CI Plan

CI is part of the replacement gate. OpenDesk should not depend on manual discipline alone for privacy, docs consistency, generated scripts, or application behavior.

## CI Principles

- Public-repo safety must be checked automatically.
- Documentation changes must remain internally consistent.
- Generated client delivery scripts must be testable without touching production.
- Application code must eventually have unit, integration, migration, and security checks.
- CI should run on pull requests and pushes to `main`.
- CI must never require production secrets.

## Initial Documentation CI

Before application code exists, CI should cover:

| Check | Purpose | Required Before |
|---|---|---|
| Markdown lint | Keep docs readable and consistent | First PR after docs baseline |
| Link check | Catch broken internal links | First PR after docs baseline |
| Sensitive-string scan | Prevent private details in public repo | Immediately |
| Ignore boundary check | Ensure `local/` and `upstream/` stay ignored | Immediately |
| Requirements reference check | Ensure validation docs reference requirement IDs over time | Before implementation starts |
| File size check | Fail on files exceeding soft limits unless the script is intentionally updated with a justified exception | Before implementation starts |
| Canonical naming check | Catch banned vague filenames and obvious internal shim/legacy terms | Before implementation starts |
| Public content hygiene check | Prevent private-workflow markers in committed project content | Immediately |

Suggested tools:

- `markdownlint-cli2`
- `lychee` or equivalent link checker
- `gitleaks` or `trufflehog` for secret scanning
- A small repo-local shell script for project-specific denylist checks

## Application CI

Once the app is scaffolded, CI should add:

| Check | Purpose |
|---|---|
| Go/Rust/Python/Node formatting | Enforce consistent code format based on chosen stack |
| Unit tests | Validate core models, token handling, RBAC, config generation |
| Integration tests | Validate API workflows against test database |
| Migration tests | Apply migrations up/down or forward-only on fresh and fixture databases |
| Static analysis | Catch common bugs and unsafe patterns |
| Dependency audit | Surface known vulnerable dependencies |
| Container build | Ensure deployable image builds |
| Compose smoke test | Start app and run health checks in CI |

## Client Delivery CI

Generated installer/config scripts are security-sensitive and must be tested.

CI should cover:

- Windows PowerShell script static validation.
- Linux shell script syntax validation with `shellcheck`.
- macOS shell script syntax validation when added.
- Generated scripts contain no full enrollment token in committed fixtures.
- Generated scripts include expected placeholders/config fields.
- Official installer checksum metadata format is valid.
- Script templates render deterministically from test config.

## Security CI

Required checks:

- Secret scan on every PR/push.
- Project-specific sensitive scan for real deployment markers.
- Dependency vulnerability scan.
- Generated artifact scan.
- Test that sample configs do not contain real domains, IPs, keys, or tokens.

The project-specific scan should reject:

- Real owner domain strings.
- Private LAN IPs outside examples.
- PEM private-key block markers.
- Obvious token/password assignments.
- RustDesk server private key filenames with included contents.

## Release CI

Before a release or cutover candidate:

- Full unit/integration suite passes.
- Container image builds.
- Migration tests pass.
- Backup/restore test passes in CI or documented staging environment.
- Docs link check passes.
- Privacy/secret scans pass.
- Validation matrix evidence has been updated.

## CI Bootstrap Tasks

- [x] Add `.github/workflows/docs.yml`.
- [x] Add `.github/workflows/security.yml`.
- [x] Add project-specific sensitive scan script.
- [x] Add initial docs checking script.
- [ ] Add markdown/link checking config.
- [ ] Add generated script tests once templates exist.
- [ ] Add application test workflow once stack is scaffolded.
- [x] Add source file size warning check.
- [x] Add canonical naming/anti-shim review check.
- [x] Add public content scan.

## Current Workflows

- `.github/workflows/docs.yml` runs `scripts/docs-check.sh`.
- `.github/workflows/security.yml` runs `scripts/privacy-scan.sh` and `scripts/public-content-scan.sh`.

The current docs check validates required docs, local Markdown links, requirement-to-validation traceability, ignored private/reference boundaries, soft file-size limits, shell syntax, and obvious anti-shim filename drift. The current security workflow runs the project-specific privacy scan across tracked and unignored files, leaves ignored private/reference paths unread, and rejects private-workflow markers in public content.

These remain bootstrap checks. They should be expanded with dedicated Markdown linting, release-grade secret scanning, dependency scanning, generated-script tests, and application test workflows as soon as the project has its first implementation slice.
