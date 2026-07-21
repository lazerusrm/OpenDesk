# Operations Runbook

This runbook covers the current OpenDesk operational boundary. It does not replace
validation evidence or a site-specific deployment procedure.

## Health and diagnostics

- Open `/status` as an authenticated operator to inspect DNS/TCP probes for the
  configured RustDesk ID and relay services and the public-key fingerprint.
- Open `/status/diagnostics.json` for a non-sensitive machine-readable summary.
  It reports database state, probe state, timestamp, and backup readiness; it
  does not return configured hosts, keys, tokens, or other configuration values.
- A failed OpenDesk database check is an OpenDesk incident. A failed RustDesk
  probe is a reachability signal, not proof of WAN, UDP, NAT, or client-session
  behavior. Validate those paths from the required client networks.

## Backup and restore

Backups are JSON exports from `/backup/export.json`. They contain inventory,
server configuration, enrollment-token hashes, and user password hashes. They
exclude sessions, audit events, and endpoint check-ins. Protect exports like
credentials and do not place them in the repository or a public web directory.

Before restore, verify the schema version, sensitivity metadata, and destination
permissions. Restore is destructive: it replaces inventory, configuration,
users, and enrollment tokens. The application validates references, duplicate
identifiers, roles, and server configuration before opening the replacement
transaction. Restore into a disposable instance first, then verify login,
server configuration, device inventory, enrollment behavior, and health checks.

The application does not silently run a backup scheduler. Set
`OPENDESK_BACKUP_SCHEDULE` and `OPENDESK_BACKUP_DIR` only when an approved
external runner is configured to invoke the authenticated export and protect
its output. `/status` must report `configured_external_runner`; otherwise the
state is `manual_only` or `incomplete` and is not a production backup claim.

## Upgrade and rollback

1. Export and verify a backup before changing the image.
2. Record the candidate image/version and confirm the persistent data volume is
   attached.
3. Start the candidate with the existing data directory. Startup applies
   committed migrations before serving requests.
4. Check `/health`, authenticated `/status`, and diagnostics JSON. Confirm login,
   inventory, server config, enrollment, and audit behavior.
5. If validation fails, stop only OpenDesk, restore the last known-good image,
   and rerun the checks. RustDesk OSS ID/relay services are independent and
   should remain available.
6. If data was changed by an incompatible migration, stop the candidate and
   restore the verified backup into a fresh data volume; do not hand-edit the
   SQLite database. Record the validation result and rollback decision.

Cutover remains blocked until S-006 through S-009 and the required backup/restore
and rollback evidence are recorded with date, environment, and artifact paths.
