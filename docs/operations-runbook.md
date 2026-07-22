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

## Migration dry-run report

The migration path is report-only. It does not provide an HTTP import endpoint and
there is no write operation. Run the separate binary against a copied or otherwise
approved current SQLite file and a sanitized export:

```text
opendesk-migration-dry-run \
  --input sanitized-export.json \
  --database opendesk.sqlite \
  --map GROUP_ID:SITE_UUID
```

Repeat `--map` for each group that has explicit operator evidence for its target
site. Group IDs accept only ASCII letters, digits, `.`, `_`, and `-`; the site ID
must be a lowercase canonical UUID. Unknown groups, unknown site UUIDs, duplicate
mappings, unsafe mapping values, unknown JSON fields, unsupported schema versions,
and any credential/token/hash/key/secret field are rejected. The report is JSON on
stdout and failure is JSON on stdout with a non-zero exit status. It contains
reconciliation actions and reasons only; it does not print credentials, password
or enrollment-token hashes, sessions, audit events, endpoint check-ins, keys, or
server configuration.

The report can be checked against an owner-approved manifest before it is reviewed:

```text
opendesk-migration-dry-run \
  --input sanitized-export.json \
  --database staging-copy.sqlite \
  --manifest approved-migration-manifest.json
```

Manifest version 1 requires lowercase SHA-256 digests for the exact source export
and serialized dry-run report, expected counts for users, groups, devices,
address books, entries, and explicit cross-group edges, a non-empty approver, canonical
approval and expiry timestamps, and explicit source-ID dispositions. Import/map/
merge dispositions require a canonical target UUID; imports for address books
and cross-group edges use a deterministic planned UUID derived from structured
source identity, and that planned UUID must not exist in the snapshot. Arbitrary
and wrong-kind targets are rejected. Retire/defer dispositions require an
explicit JSON `null` target. User dispositions must state role and credential
paths; those fields are rejected for every other source kind. The CLI
requires every source entity and explicit cross-group edge ID to appear exactly
once; edge counts are never inferred from memberships. Unknown or recursively
sensitive fields (password/hash/token/secret/key/session/audit/topology) are
rejected. The manifest is only a report precondition: this command has no apply
or database-write operation.


```text
sqlite3 source.db '.backup staging-copy.sqlite'
opendesk-migration-dry-run --input sanitized-export.json --database staging-copy.sqlite
```

Alternatively, perform a verified checkpoint and offline copy under your
approved maintenance procedure, then confirm `staging-copy.sqlite-wal` is absent
or empty before running the command. An existing nonempty `-wal` sidecar is
rejected to prevent an incomplete snapshot. The command reports a generic JSON
error directing the operator to a verified backup artifact.


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
