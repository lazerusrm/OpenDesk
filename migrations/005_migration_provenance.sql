ALTER TABLE users ADD COLUMN activation_state TEXT NOT NULL DEFAULT 'active'
    CHECK (activation_state IN ('active', 'disabled'));

CREATE TABLE opendesk_instance (
    instance_uuid TEXT PRIMARY KEY NOT NULL
);

CREATE TABLE migration_runs (
    run_id TEXT PRIMARY KEY NOT NULL,
    source_system TEXT NOT NULL,
    source_instance TEXT NOT NULL,
    source_export_id TEXT NOT NULL,
    source_snapshot_sha256 TEXT NOT NULL,
    input_sha256 TEXT NOT NULL,
    target_instance_uuid TEXT NOT NULL REFERENCES opendesk_instance(instance_uuid),
    target_state_sha256 TEXT NOT NULL,
    backup_sha256 TEXT NOT NULL,
    plan_sha256 TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('applied')),
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    UNIQUE (source_instance, input_sha256)
);

CREATE TABLE migration_source_bindings (
    source_system TEXT NOT NULL,
    source_instance TEXT NOT NULL,
    entity_kind TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_uuid TEXT NOT NULL,
    run_id TEXT NOT NULL REFERENCES migration_runs(run_id) ON DELETE RESTRICT,
    PRIMARY KEY (source_system, source_instance, entity_kind, source_id)
);

CREATE TABLE migration_activation_tokens (
    activation_token_uuid TEXT PRIMARY KEY NOT NULL,
    user_uuid TEXT NOT NULL UNIQUE REFERENCES users(user_uuid) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    run_id TEXT NOT NULL REFERENCES migration_runs(run_id) ON DELETE RESTRICT,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_migration_activation_tokens_expiry
    ON migration_activation_tokens(expires_at);
