# CODEX.md

Instructions for Codex and other coding agents working in this repository.

## Project Context

OpenDesk is intended to be a public repository. Treat all committed content as public by default.

The project builds a clean-room management/control plane around RustDesk OSS. The default architecture keeps official RustDesk clients and OSS `hbbs`/`hbbr` for remote desktop transport, while OpenDesk provides inventory, address book, deployment/config generation, monitoring, backups, and admin workflows.

## Privacy Rules

Never commit real deployment details, including:

- Real domains.
- Public IP addresses.
- LAN IP addresses.
- Hostnames that identify private infrastructure.
- SSH usernames or connection strings.
- RustDesk server private keys.
- RustDesk server public keys or fingerprints unless explicitly marked safe for docs/tests.
- Enrollment tokens.
- API keys.
- Passwords.
- Session cookies.
- Captured traffic.
- Customer names, site names, device names, or endpoint IDs from production.
- Screenshots showing private infrastructure, device lists, or credentials.

Use placeholders in committed docs and examples:

- `rd.example.com`
- `rd-admin.example.com`
- `LAN_HOST`
- `DEVICE_ID_EXAMPLE`
- `TOKEN_EXAMPLE`
- `SITE_EXAMPLE`

Private local notes may be stored only in ignored paths:

- `local/`
- `PRIVATE.md`
- `*.private.md`
- `.env`
- `*.env`

Future agents should check `local/README.md` and `local/site/context.private.md` when they need deployment context. These files are intentionally ignored and may contain real site-specific values. Do not quote those values into committed files or public issue/PR text.

Before committing, run a sensitive-string scan. At minimum, check for real domains, LAN ranges, passwords, secrets, tokens, and private hostnames.

Example:

```bash
rg -n "industrialcamera|192\.168|10\.|172\.(1[6-9]|2[0-9]|3[0-1])|password|secret|token|PRIVATE KEY|BEGIN .*KEY" .
```

Review matches manually. Some words like `token` may be legitimate in docs, but real values must not be committed.

## Engineering Rules

- Keep the first implementation focused on the external control plane.
- Do not fork RustDesk client/server code unless a specific workflow requires it and the tradeoff is documented.
- Prefer official RustDesk clients for signing, mobile apps, and updater behavior.
- Do not modify official RustDesk binaries in the first build slices.
- Avoid reverse-engineering RustDesk Pro as the primary strategy.
- Do not store unattended access passwords in plaintext.
- If password storage is later required, prefer an external secret manager integration before building native storage.
- Keep production values in runtime config, not source.
- Keep generated installers/scripts auditable and avoid embedding long-lived secrets.
- Default to SQLite for early local work unless requirements justify Postgres.
- Keep deployment simple: one app service, one database, one reverse proxy path.
- Follow `docs/engineering-standards.md`: reject internal shims, compatibility creep, vague naming, and monolithic files.
- Keep producer-to-consumer contracts canonical. If RustDesk or another external system uses different names, isolate that mapping at the boundary.

## Security Model Reminder

OpenDesk dashboard access control is not automatically RustDesk session enforcement.

If a user has a RustDesk device ID and valid unattended password, OSS RustDesk may still allow a connection regardless of OpenDesk dashboard permissions. Any claim of enforced access control must be backed by endpoint settings, credential rotation, network restrictions, or explicit protocol/client integration.

## Documentation Rules

- Keep examples generic and public-safe.
- Use `example.com` domains in committed docs.
- Do not include real server keys, fingerprints, endpoint IDs, or customer/device names.
- If documenting a production procedure, split it into:
  - public generic procedure in the repo
  - private deployment-specific values in ignored local notes

## Validation Expectations

Every feature should eventually map to a validation case in `docs/validation-matrix.md`.

For client delivery work, passing criteria must include:

- No executable renaming as the primary workflow.
- Official binary signatures/checksums preserved where applicable.
- No plaintext secrets in generated scripts.
- Endpoint registration works with scoped, revocable enrollment tokens.
