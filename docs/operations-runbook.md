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
- Bind `OPENDESK_LISTEN_ADDR` to loopback and terminate TLS plus official-client `/api` on a reverse proxy. Do not publish the application listen port.

## Official-client token key

Set `OPENDESK_CLIENT_TOKEN_HMAC_KEY` to a stable random hexadecimal secret of at
least 32 bytes before starting OpenDesk. Store it outside the repository with the
same availability controls as other deployment secrets. OpenDesk stores only
HMAC-SHA-256 token digests; losing or rotating this key invalidates all official-
client login tokens and requires users to log in again. Logical backups exclude
client access tokens.

## Transport admission

Transport admission is disabled unless `OPENDESK_TRANSPORT_INTROSPECTION_KEY`
is configured. Provision the same random hexadecimal secret of at least 32 bytes
to the authorized transport process as `OPENDESK_TRANSPORT_INTROSPECTION_KEY`;
never pass it on a command line or place it in the repository. Configure the
transport process with `OPENDESK_TRANSPORT_INTROSPECTION_URL` using HTTPS, or
loopback HTTP only when both processes share a host. Authorization fails closed
when OpenDesk is unavailable and requires an active client token plus explicit
device visibility. This implementation evidence does not replace direct/relay
session validation or authorize production cutover.

## Backup and restore

Backups are versioned JSON exports from `/backup/export.json`. They contain inventory,
server configuration, enrollment-token hashes, user password hashes and activation
states, scoped address-book sharing rules, access-group access grants, and address-book tags. They
exclude browser sessions, official-client access tokens, audit events, and endpoint check-ins. Protect exports like
credentials and do not place them in the repository or a public web directory.

Before restore, verify the schema version, sensitivity metadata, and destination
permissions. Restore is destructive: it replaces inventory, configuration,
users, enrollment tokens, and migration provenance. Runtime client tokens are not
restored. The application validates references, duplicate
identifiers, roles, and server configuration before opening the replacement
transaction. Restore into a disposable instance first. Every official client must
log in again after restore because runtime client tokens are deliberately excluded
and revoked by replacement. Then verify login, server configuration, device
inventory, enrollment behavior, and health checks.

The application does not silently run a backup scheduler. Set
`OPENDESK_BACKUP_SCHEDULE` and `OPENDESK_BACKUP_DIR` only when an approved
external runner is configured to invoke the authenticated export and protect
its output. `/status` must report `configured_external_runner`; otherwise the
state is `manual_only` or `incomplete` and is not a production backup claim.

## Sanitized RustDesk Pro source export

`export-rustdesk-pro-sanitized.py` is a read-only external-boundary adapter for a
verified RustDesk Pro SQLite snapshot. It writes the versioned sanitized contract
to stdout; redirect it only to protected, non-repository storage. It never exports
password hashes, TFA, sessions, keys, tokens, setting values, free-form notes, or
endpoint metadata other than a peer's `device_name` alias.

```text
python3 scripts/export-rustdesk-pro-sanitized.py \
  --database pro-snapshot.sqlite \
  --source-instance approved-source-instance \
  --source-export-id approved-export-id \
  --source-schema-version 1.7.5 \
  --role-map 0:admin \
  --role-map 1:operator \
  > sanitized-export.json
```

The adapter opens the database with SQLite immutable read-only mode. Use
`--unsupported-inventory` first to emit only populated unsupported-table names and
counts, without reading or exporting their contents. This supports an owner parity
review without exposing passwords, installers, settings, or other opaque payloads.
Normal export rejects unmapped source roles, dangling user/group or address-book/device
references, ownerless books, source device-group assignments, populated address-book
rules, control-role mappings, custom clients, strategies, role scopes, user roles, and
unreviewed setting values. A category can be emitted only as count-only `retired`
metadata by repeating `--retire-unsupported custom_client`, `strategy`, or `settings`.
Every populated category must be named; unretired categories still fail closed. The
retirement metadata is bound to the export digest, preflight, manifest digest, and
separately signed apply plan. It records no payload, does not preserve behavior, and
must not be used to claim client, session, or policy equivalence. The adapter does not
interpret those records or silently broaden
access. This is deliberate: custom-client definitions can carry endpoint passwords
and opaque installers, and strategy options can claim RustDesk session controls
that OpenDesk does not enforce. Treat the resulting export as sensitive operational
metadata even though it contains no credentials.

## Migration preflight

Before creating the preflight, explicitly mark the disposable staging database
inside its own SQLite file. The apply command refuses any unmarked target; never
place this marker in a production database:

```text
sqlite3 staging-opendesk.sqlite \
  "INSERT INTO migration_staging_targets (instance_uuid, marked_at)
   SELECT instance_uuid, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
   FROM opendesk_instance;"
```

After the marker is present, run the read-only preflight:

```text
opendesk-migration-preflight \
  --input sanitized-export.json \
  --database staging-opendesk.sqlite \
  --backup staging-opendesk-before-import.sqlite
```

The command is read-only. It has no `--apply` option, never runs schema
migrations, and never creates a database. It rejects non-regular or active-WAL
files, an uninitialized target, and a backup whose durable OpenDesk instance
identity differs from the target. Its JSON result contains only source/target and
backup SHA-256 bindings and must be included in the separately signed staging
apply approval.

The apply verifier key is a protected target-side deployment setting named
`OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX`; it is not accepted as an apply
artifact. Provision the approved 32-byte Ed25519 verifying key through the
existing secret-management path before running the command. Keep its private
signing counterpart separate from the staging target and never place either
key in the repository. After a successful staging import, accounts remain disabled unless credentials
were attached through the existing signed in-apply bridge. For the dedicated
post-apply path, place the separately generated protected credential artifact in
owner-only non-repository storage with mode `0600`, independently verify its
lowercase SHA-256 digest and provenance, then run:

```text
opendesk-migration-credentials-attach \
  --attach --activate-all \
  --database staging-opendesk.sqlite \
  --credentials protected-credentials.json \
  --expected-sha256 approved-lowercase-sha256
```

The command opens only an existing owner-only regular database, binds the exact
raw artifact digest and source/run/input/target provenance to an already applied
migration, requires the complete unique imported-user binding set and every
account still disabled, and atomically stores the one-login bcrypt bridge,
activates all bound users, and records an attachment receipt. Any mismatch,
partial prior attachment, active user, or replay rolls back without activation.
Its output is static JSON and never includes artifact content or verifier values.
If protected attachment is not approved, an administrator must instead set a new
password and activate each disabled imported account from `/users`; that action
is audited and never accepts a source password or hash.

## Operator account password reset
Run only from an interactive controlling terminal as the database owner. The existing
SQLite file must be regular, single-link, and inaccessible to group and others:
```text
opendesk-user-password-reset \
  --database /protected/path/opendesk.sqlite \
  --username exact-account-name \
  --reset
```
The command prompts twice without echo and never accepts the password from arguments,
environment, or standard input. It requires an active exact-match account, applies the
eight-character account policy, and emits static JSON. Success atomically writes the
Argon2 hash, consumes a pending migration credential, deletes browser sessions, revokes
client tokens, and records a detail-free `operator_cli` `password_reset` audit event.
Failure leaves account and access state unchanged.
## Migration dry-run report

The migration dry-run command is report-only. It does not provide an HTTP import
endpoint and never writes to the database. Run the separate binary against a copied
or otherwise approved current SQLite file and a sanitized export:

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

Cutover remains blocked until S-006 through S-009 and the required backup/restore and rollback evidence are recorded with date, environment, and artifact paths.
