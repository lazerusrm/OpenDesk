CREATE TABLE migration_staging_targets (
    instance_uuid TEXT PRIMARY KEY NOT NULL REFERENCES opendesk_instance(instance_uuid) ON DELETE RESTRICT,
    marked_at TEXT NOT NULL
);
